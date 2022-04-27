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

impl ShoppingCart {
    pub const STATUS_READY: &'static str = "ready";
    pub const STATUS_PURCHASING: &'static str = "purchasing";
    pub const STATUS_SUCCESS: &'static str = "success";
    pub const STATUS_FAIL: &'static str = "fail";

    pub const OP_ADD: u8 = 1;
    pub const OP_SUB: u8 = 2;
    pub const OP_IMMEDIATELY_PURCHASE: u8 = 3;
    pub const OP_YUYUE: u8 = 4;
    pub const OP_PURCHASE_SUCCESS: u8 = 5;
    pub const OP_PURCHASE_FAIL: u8 = 6;
    pub const OP_YUYUE_ING: u8 = 7;
    pub const OP_IN_STOCK: u8 = 8;
    // 有货
    pub const OP_OUT_OF_STOCK: u8 = 9; // 无货
}

pub async fn add_goods_cart(code: String, req: AddGoodsCartReq) -> Result<()> {
    req.validate()?;
    let u = token::get_cached_user(code.as_str(), false).await?;
    let w = DB_CLIENT.new_wrapper()
        .eq("sku", req.sku.as_str())
        .eq("user_id", u.id);
    let tx = DB_CLIENT.begin_tx().await?;
    let rdata = DB_CLIENT.fetch_by_wrapper::<Option<ShoppingCart>>(tx.as_str(), &w).await?;
    if let Some(mut data) = rdata {
        data.name = req.name;
        data.purchase_num = req.purchase_num;
        data.ori_price = req.ori_price;
        data.cur_price = req.cur_price;
        data.is_delete = 0;
        data.is_stock = req.is_stock;
        data.yuyue_dt = req.yuyue_dt;
        data.yuyue_start_dt = None;
        data.yuyue_end_dt = None;
        data.purchase_url = req.purchase_url;
        data.purchase_type = req.purchase_type;
        data.update_time = Some(PKLocal::now());
        data.status = "ready".to_owned();
        DB_CLIENT.update_by_id::<ShoppingCart>(tx.as_str(), &mut data).await?;
        DB_CLIENT.commit(tx.as_str()).await?;
        return Ok(());
    }
    let mut pa = ShoppingCart {
        user_id: u.id,
        sku: req.sku,
        name: req.name,
        purchase_num: req.purchase_num,
        purchase_type: req.purchase_type,
        ori_price: req.ori_price,
        cur_price: req.cur_price,
        purchase_url: req.purchase_url,
        is_stock: req.is_stock,
        status: ShoppingCart::STATUS_READY.to_string(),
        ..Default::default()
    };
    if req.yuyue_dt.is_some() {
        pa.yuyue_dt = req.yuyue_dt;
    }
    DB_CLIENT.save(tx.as_str(), &pa).await?;
    DB_CLIENT.commit(tx.as_str()).await?;
    Ok(())
}


pub async fn update_goods_cart(code: String, id: IDType, req: UpdateGoodsCartReq) -> Result<()> {
    req.validate()?;
    let u = token::get_cached_user(code.as_str(), false).await?;
    let mut args = Vec::with_capacity(3);
    let mut sqlv = Vec::with_capacity(3);
    let mut sql = String::with_capacity(256);
    sql.push_str("update shopping_cart set ");
    let mut where_sql = " where is_delete = 0 and user_id=? and id= ? ".to_string();
    let mut where_args = vec![json!(u.id), json!(id)];
    match req.op {
        ShoppingCart::OP_ADD => {
            sql.push_str(" purchase_num = purchase_num + 1,");
        }
        ShoppingCart::OP_SUB => {
            sql.push_str(" purchase_num = purchase_num - 1,");
            // 保证 purchase_num  不会 小于 0，引起错误
            where_sql += " and purchase_num > 1 ";
        }
        ShoppingCart::OP_IMMEDIATELY_PURCHASE => {
            sql.push_str(" status = 'purchasing',");
        }

        ShoppingCart::OP_YUYUE => {
            // sql.push_str( " status = 'purchasing', yuyue_dt=?, yuyue_start_dt=?,yuyue_end_dt=?, ");
            sql.push_str(" yuyue_dt=?, ");
            args.push(json!(datetime_fmt(&req.yuyue_dt.unwrap())))
        }
        ShoppingCart::OP_PURCHASE_SUCCESS => {
            sql.push_str(" status = 'success', ");
        }
        ShoppingCart::OP_PURCHASE_FAIL => {
            sql.push_str(" status = 'fail', ");
        }

        ShoppingCart::OP_YUYUE_ING => {
            sql.push_str(" status = 'yuyueing',");
        }
        ShoppingCart::OP_IN_STOCK => {
            sql.push_str(" is_stock = 1,");
        }
        ShoppingCart::OP_OUT_OF_STOCK => {
            sql.push_str(" is_stock = 0,");
        }
        _ => {
            unreachable!()
        }
    }
    let tx = DB_CLIENT.begin_tx().await?;
    sqlv.push(" update_time=? ");
    args.push(json!(datetime_fmt(&PKLocal::now())));
    sql.push_str(sqlv.join(",").as_str());
    sql.push_str(&mut where_sql);
    args.append(&mut where_args);
    DB_CLIENT.exec_prepare(tx.as_str(), sql.as_str(), &args).await?;
    match DB_CLIENT.commit(tx.as_str()).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("{:?}", e);
            DB_CLIENT.rollback(tx.as_str()).await?;
            Ok(())
        }
    }
}

pub async fn update_yuyue_goods_cart(code: String, id: IDType, req: UpdateYuyueGoodsCartReq) -> Result<()> {
    req.validate()?;
    if req.op != ShoppingCart::OP_YUYUE {
        return Ok(());
    }
    let u = token::get_cached_user(code.as_str(), false).await?;
    let sql = "update shopping_cart \
            set  yuyue_dt=?, status='ready', yuyue_start_dt=?, yuyue_end_dt=?, update_time=? \
            where is_delete = 0 and user_id=? and id= ? ;";
    let args = vec![
        json!(datetime_fmt(&req.yuyue_dt.unwrap())),
        json!(datetime_fmt(&req.yuyue_start_dt.unwrap())),
        json!(datetime_fmt(&req.yuyue_end_dt.unwrap())),
        json!(datetime_fmt(&PKLocal::now())),
        json!(u.id),
        json!(id),
    ];
    DB_CLIENT.exec_prepare("", sql, &args).await?;
    Ok(())
}

pub async fn update_cart_goods_purchase_url(code: String, req: UpdateGoodsCartUrlReq) -> Result<()> {
    req.validate()?;
    let u = token::get_cached_user(code.as_str(), false).await?;
    let sql = "update shopping_cart set purchase_url=?, update_time=?  where user_id=? and sku =? and is_delete=0 ";
    let args = vec![json!(req.purchase_url), json!(datetime_fmt(&PKLocal::now())), json!(u.id), json!(req.sku)];
    let tx = DB_CLIENT.begin_tx().await?;
    DB_CLIENT.exec_prepare(tx.as_str(), sql, &args).await?;
    DB_CLIENT.commit(tx.as_str()).await?;
    Ok(())
}

pub async fn delete_goods_cart(code: String, id: IDType) -> Result<()> {
    let u = token::get_cached_user(code.as_str(), false).await?;
    // 物理删除
    // let sql = "update shopping_cart set is_delete=1 where id = ?";
    let sql = "delete from shopping_cart where id = ? and user_id = ?";
    DB_CLIENT.exec_prepare("", sql, &vec![json!(id), json!(u.id)]).await?;
    Ok(())
}

pub async fn batch_delete_goods_carts(code: String, req: BatchDeleteGoodsCartReq) -> Result<()> {
    req.validate()?;
    let u = token::get_cached_user(code.as_str(), false).await?;
    // 物理删除
    let sql;
    let args;
    if let Some(ref ids) = req.ids {
        // 删除某些
        // sql = "update shopping_cart set is_delete=1 where id in (?) and user_id = ?";
        sql = "delete from shopping_cart where id in (?) and user_id = ?";
        args = vec![json!(ids.iter().map(|id|id.to_string()).collect::<Vec<String>>().join(",")),
                    json!(u.id)];
    } else {
        // None时删除全部
        // sql = "update shopping_cart set is_delete=1 where user_id = ?";
        sql = "delete from shopping_cart where user_id = ?";
        args = vec![json!(u.id)];
    }
    DB_CLIENT.exec_prepare("", sql, &args).await?;
    Ok(())
}

pub async fn list_goods_carts(code: String, req: KeyWordPageReq) -> Result<Page<ShoppingCart>> {
    req.validate()?;
    let u = token::get_cached_user(code.as_str(), false).await?;
    let page_size = req.page_size.unwrap_or(50);
    let page_no = req.page_no.unwrap_or(1);
    let page_req = PageRequest::new(page_no, page_size);//分页请求，页码，条数
    let mut w = DB_CLIENT.new_wrapper()
        .eq("user_id", u.id)
        .eq("is_delete", 0);
    debug!("{:?}", req);
    if let Some(ref kw) = req.key_word {
        let kw = kw.trim();
        if kw.len() > 0 {
            // (sku like 'aaa%' || name like 'aaa%')
            w = w.and().push_sql("(")
                .like_right("sku", kw)
                .or()
                .like_right("name", kw)
                .push_sql(")");
        }
    }
    w = w.order_by(false, &["id"]);
    let data: Page<ShoppingCart> = DB_CLIENT.fetch_page_by_wrapper(
        "", &w, &page_req).await?;
    Ok(data)
}