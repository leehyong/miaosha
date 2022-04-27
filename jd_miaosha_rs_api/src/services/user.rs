use redis::AsyncCommands;
use validator::Validate;
use log::{error, debug};
use crate::{get_redis_conn, DB_CLIENT, IDType};
use crate::error::*;
use crate::utils::{token, datetime_fmt};
use crate::models::*;
use crate::req::*;
use crate::*;
use chrono::Duration;
use serde::Deserialize;
use serde_json::{Value, json};
use rbatis::crud::CRUD;
use rbatis::plugin::page::{Page, PageRequest};


pub async fn create_user_activate_code(level: u8) -> Result<String> {
    let mut user = User {
        vip_level: level.into(),
        ..Default::default()
    };
    let tx = DB_CLIENT.begin_tx().await?;
    DB_CLIENT.save(tx.as_str(), &user).await?;
    // 在同一个事务里，查询最新插入的数据
    user = DB_CLIENT.fetch(
        tx.as_str(), "select * from user where id = (select LAST_INSERT_ID())").await?;
    let user_id = user.id.unwrap();
    if let Some(activate_code) = token::create_token(user_id, level) {
        user.activate_code = activate_code;
        let user_info = cached_user_in_str(user_id, level, "");
        let mut con = get_redis_conn().await?;
        let w = DB_CLIENT.new_wrapper().eq("id", Some(user_id));
        // 更新激活码
        user.update_time = Some(PKLocal::now());
        DB_CLIENT.update_by_wrapper(tx.as_str(), &mut user, &w, false).await?;
        let key = token::activate_code_key(user.activate_code.as_str());
        con.set(key, user_info).await?;
        DB_CLIENT.commit(tx.as_str()).await?;
        return Ok(serde_json::json!({"activate_code":user.activate_code.as_str()}).to_string());
    }
    DB_CLIENT.rollback(tx.as_str()).await?;
    Err(MiaoshaError::OpError(EOpError::CreateActivationCode(level)))
}

fn cached_user_in_str(user_id: IDType, level: u8, expire_dt: &str) -> String {
    serde_json::json!(
        {
            "id":user_id,
            "vip_level":level,
            "expire_dt":expire_dt,
        }
    ).to_string()
}

pub async fn activate(req: ActivateReq) -> Result<()> {
    req.validate()?;
    let user = token::get_cached_user(req.activate_code.as_str(), true).await?;
    if user.expire_dt.is_some() {
        // 防止重复激活
        warn!("Can't activate repeatedly: {}", req.activate_code.as_str());
        return Ok(());
    }
    let seconds = user.vip_level.vip_level_seconds();
    let now = PKLocal::now();
    let exp_dt = now.checked_add_signed(Duration::seconds(seconds)).unwrap_or(now);
    let exp_dt_str = datetime_fmt(&exp_dt);
    let user_info = cached_user_in_str(user.id, user.vip_level.into_u8(), exp_dt_str.as_str());
    let key = token::activate_code_key(req.activate_code.as_str());
    let mut con = get_redis_conn().await?;
    info!("Activating:{} {}-{}", req.activate_code.as_str(), user.id, user.vip_level.into_u8());
    let update_sql = "update user set `mac_addr` = ?, `expire_dt`=?, `update_time`=? where id = ?";
    let args = vec![json!(req.mac.as_str()), json!(exp_dt_str.as_str()), json!(datetime_fmt(&now)), json!(user.id)];
    let vip_history = VipHistory {
        id: None,
        user_id: user.id,
        vip_level: user.vip_level,
        start_dt: Some(now.clone()),
        expire_dt: Some(exp_dt),
        create_time: Some(now.clone()),
    };
    let login_history = LoginHistory {
        id: None,
        user_id: user.id,
        mac_addr: req.mac,
        create_time: Some(now.clone()),
    };
    let tx = DB_CLIENT.begin_tx().await?;
    // 1 新增 vip 激活历史记录
    DB_CLIENT.save(tx.as_str(), &vip_history).await?;
    // 2 新增登录历史记录
    DB_CLIENT.save(tx.as_str(), &login_history).await?;
    // 3 更新激活的 mac 地址和 过期时间
    DB_CLIENT.exec_prepare(tx.as_str(), update_sql, &args).await?;
    // 4 把 redis 操作放在最后，这样就不会破坏事务的完整性了,
    //   如果redis操作失败，则前面的数据库操作会被回滚;若redis操作成功，则整个事务会被正确提交
    con.set_ex(key, user_info, seconds as usize).await?;
    DB_CLIENT.commit(tx.as_str()).await?;
    info!("Activating:{} {}-{}, expired:{}", req.activate_code.as_str(), user.id, user.vip_level.into_u8(), exp_dt_str.as_str());
    Ok(())
}

pub async fn refresh(req: RefreshReq) -> Result<()> {
    req.validate()?;
    for code in req.codes.iter() {
        //
        if let Err(MiaoshaError::OpError(EOpError::ActivationCodeExpired(_))) = token::get_cached_user(code.as_str(), true).await {
            let w = DB_CLIENT.new_wrapper()
                .eq("activate_code", code.as_str())
                .limit(1);
            let tx = DB_CLIENT.begin_tx().await?;
            let user =  DB_CLIENT.fetch_by_wrapper::<Option<User>>(tx.as_str(), &w).await?;
            if let Some(user) = user{
                let seconds = user.vip_level.vip_level_seconds();
                let now = PKLocal::now();
                let exp_dt = now.checked_add_signed(Duration::seconds(seconds)).unwrap_or(now);
                let exp_dt_str = datetime_fmt(&exp_dt);
                let id = user.id.unwrap();
                let level = user.vip_level.into_u8();
                let user_info = cached_user_in_str(id, level, exp_dt_str.as_str());
                let key = token::activate_code_key(code.as_str());
                let mut con = get_redis_conn().await?;
                info!("Refreshing:{} {}-{}", code.as_str(), id, level);
                let update_sql = "update user set `expire_dt`=?, `update_time`=? where id = ?";
                let args = vec![json!(exp_dt_str.as_str()), json!(datetime_fmt(&now)), json!(user.id)];
                let vip_history = VipHistory {
                    id: None,
                    user_id: id,
                    vip_level: user.vip_level,
                    start_dt: Some(now.clone()),
                    expire_dt: Some(exp_dt),
                    create_time: Some(now.clone()),
                };
                // 1 新增 vip 激活历史记录
                DB_CLIENT.save(tx.as_str(), &vip_history).await?;
                // 2 更新激活的 mac 地址和 过期时间
                DB_CLIENT.exec_prepare(tx.as_str(), update_sql, &args).await?;
                // 3 把 redis 操作放在最后，这样就不会破坏事务的完整性了,
                //   如果redis操作失败，则前面的数据库操作会被回滚;若redis操作成功，则整个事务会被正确提交
                con.set_ex(key, user_info, seconds as usize).await?;
                DB_CLIENT.commit(tx.as_str()).await?;
                info!("Refreshed:{} {}-{}, expired:{}", code.as_str(), id, level, exp_dt_str.as_str());
            }
        }else{
            warn!("{}还未过期，不用重复激活", code.as_str());
        }
    }
    Ok(())
}

pub async fn add_account(code: String, req: AddAccountReq, vip_users:&Vec<u32>) -> Result<()> {
    req.validate()?;
    let u = token::get_cached_user(code.as_str(), false).await?;
    let max_account_num = u.vip_level.vip_level_max_accounts(vip_users);
    let tx = DB_CLIENT.begin_tx().await?;
    let cnt_sql = "select count(*) as cnt from platform_account where user_id = ?";
    let mut result: serde_json::Value = DB_CLIENT.fetch_prepare(tx.as_str(), cnt_sql, &vec![json!(u.id)]).await?;
    let cnt;
    if result.is_array() {
        cnt = result[0].get("cnt").unwrap_or(&serde_json::json!(0)).as_i64().unwrap_or(0) as usize;
    } else {
        cnt = result.get("cnt").unwrap_or(&serde_json::json!(0)).as_i64().unwrap_or(0) as usize;
    }
    debug!("{:?} user:{}  current count: {}", result, u.id, cnt);
    if cnt >= max_account_num {
        DB_CLIENT.rollback(tx.as_str()).await?;
        return Err(MiaoshaError::OpError(EOpError::ExceedMaxAccountLimits(max_account_num)));
    }
    let pa = PlatformAccount {
        user_id: u.id,
        account: req.account,
        pwd: req.pwd,
        ..Default::default()
    };
    if let Err(e) = DB_CLIENT.save(tx.as_str(), &pa).await {
        DB_CLIENT.rollback(tx.as_str()).await?;
        return Err(e.into());
    }
    DB_CLIENT.commit(tx.as_str()).await?;
    Ok(())
}


pub async fn update_account(code: String, id: IDType, req: UpdateAccountReq) -> Result<()> {
    req.validate()?;
    let u = token::get_cached_user(code.as_str(), false).await?;
    let mut args = Vec::with_capacity(3);
    let mut sql = String::with_capacity(256);
    sql.push_str("update platform_account set ");
    let mut sqlv = Vec::with_capacity(3);
    if let Some(ref arg) = req.pwd {
        sqlv.push("pwd = ?");
        args.push(json!(arg.as_str()));
    }
    if let Some(ref arg) = req.account {
        sqlv.push("account = ?");
        args.push(json!(arg.as_str()));
    }
    if let Some(ref arg) = req.cookie {
        sqlv.push("cookie = ?");
        sqlv.push("cookie_last_update_dt = ?");
        args.push(json!(arg.as_str()));
        args.push(json!(datetime_fmt(&PKLocal::now())));
    }
    if let Some(ref arg) = req.eid {
        sqlv.push("eid = ?");
        args.push(json!(arg.as_str()));
    }
    if let Some(ref arg) = req.fp {
        sqlv.push("fp = ?");
        args.push(json!(arg.as_str()));
    }
    if args.len() > 0 {
        let tx = DB_CLIENT.begin_tx().await?;
        sqlv.push(" update_time=? ");
        args.push(json!(datetime_fmt(&PKLocal::now())));
        sql.push_str(sqlv.join(",").as_str());
        sql.push_str(" where user_id=? and id= ?");
        args.append(&mut vec![json!(u.id), json!(id)]);
        DB_CLIENT.exec_prepare(tx.as_str(), sql.as_str(), &args).await?;
        DB_CLIENT.commit(tx.as_str()).await?;
    }
    Ok(())
}

pub async fn delete_account(code: String, id: IDType) -> Result<()> {
    let u = token::get_cached_user(code.as_str(), false).await?;
    // 物理删除账户
    let tx = DB_CLIENT.begin_tx().await?;
    let w = DB_CLIENT.new_wrapper().eq("id", id).eq("user_id", u.id);
    DB_CLIENT.remove_by_wrapper::<PlatformAccount>(tx.as_str(), &w).await?;
    DB_CLIENT.commit(tx.as_str()).await?;
    Ok(())
}

pub async fn batch_delete_accounts(code: String, req: BatchDeleteAccountReq) -> Result<()> {
    req.validate()?;
    let u = token::get_cached_user(code.as_str(), false).await?;
    // 物理删除账户
    let tx = DB_CLIENT.begin_tx().await?;
    let w;
    if let Some(ref ids) = req.ids {
        // 删除某些
        w = DB_CLIENT.new_wrapper().r#in("id", &ids).eq("user_id", u.id);
    } else {
        // None时删除全部
        w = DB_CLIENT.new_wrapper().eq("user_id", u.id);
    }
    DB_CLIENT.remove_by_wrapper::<PlatformAccount>(tx.as_str(), &w).await?;
    DB_CLIENT.commit(tx.as_str()).await?;
    Ok(())
}

pub async fn list_accounts(code: String, req: KeyWordPageReq) -> Result<Page<PlatformAccount>> {
    req.validate()?;
    let u = token::get_cached_user(code.as_str(), false).await?;
    let page_size = req.page_size.unwrap_or(450);
    let page_no = req.page_no.unwrap_or(1);
    let page_req = PageRequest::new(page_no, page_size);//分页请求，页码，条数
    let mut w = DB_CLIENT.new_wrapper()
        .eq("user_id", u.id);
    debug!("{:?}", req);
    if let Some(ref kw) = req.key_word {
        let kw = kw.trim();
        if kw.len() > 0 {
            w = w.like_right("account", kw);
        }
    }
    w = w.order_by(false, &["id"]);
    let tx = DB_CLIENT.begin_tx().await?;
    let data: Page<PlatformAccount> = DB_CLIENT.fetch_page_by_wrapper(
        tx.as_str(), &w, &page_req).await?;
    DB_CLIENT.commit(tx.as_str()).await?;
    Ok(data)
}

pub async fn user_info(code: String) -> Result<User> {
    let cu = token::get_cached_user(code.as_str(), false).await?;
    let u: User = DB_CLIENT.fetch_by_id("", &cu.id).await?;
    Ok(u)
}

pub async fn heartbeat(code: String, req:HeartbeatReq) -> Result<()> {
    req.validate()?;
    let cached_user = token::get_cached_user(code.as_str(), false).await?;
    let u: User = DB_CLIENT.fetch_by_id("", &cached_user.id).await?;
    if u.mac_addr.to_uppercase().eq(&req.mac.to_uppercase()){
        Ok(())
    }else{
        // 激活时的mac地址， 与心跳检查时的mac地址不一样的时候，会报 403
        Err(EOpError::UnExpectedMacAddr(u.mac_addr.to_owned(), req.mac.to_owned()).into())
    }
}


