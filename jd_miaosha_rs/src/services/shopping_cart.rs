use std::collections::{HashMap, HashSet};
use std::env;
use std::sync::Arc;

use futures::{future::join_all, FutureExt, StreamExt, TryFutureExt};
use http::{HeaderMap, StatusCode};
use log::{debug, error, info, warn};
use rand::prelude::*;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{from_slice, from_str, json, to_string, Value};
use thirtyfour::{By, Cookie, support, WebDriver, WebDriverCommands};
use thirtyfour::error::WebDriverError;
use tokio::sync::{Mutex, RwLock};
use url::form_urlencoded;

use crate::*;
use crate::error::{JdMiaoshaError, OpError, Result};
use crate::models::*;
use crate::services::driver::*;
use crate::services::goods::*;
use crate::utils::*;

const READY: u8 = 1;
const SUCCESS: u8 = 2;
const FAIL: u8 = 3;

lazy_static! {
    // static ref ACCOUNT_SHOPPING_CART_RESULT_LOCKS: Arc<Mutex<HashMap<(IDType, IDType), u8>>> =
    //     Default::default();
    static ref ACCOUNT_CART_LOCKS: Arc<Mutex<HashMap<IDType, AccountCartLockType>>> = Default::default();
    static ref ACCOUNT_CART_SKU_UUIDS: Arc<RwLock<HashMap<IDType, HashMap<String, String>>>> =
        Default::default();
}
type AccountCartLockType = Arc<Mutex<IDType>>;
type AccountCartBuyResultLockType = Arc<Mutex<bool>>;

async fn get_account_cart_lock(account_id: IDType) -> AccountCartLockType {
    // 获取用户的购物车锁
    let mut guard = ACCOUNT_CART_LOCKS.lock().await;
    guard.entry(account_id)
        .or_insert(Arc::new(Mutex::new(account_id)))
        .clone()
}

#[derive(Clone)]
pub struct ShoppingCartService;

impl ShoppingCartService {
    // pub const ADD_TO_CART_URL: &'static str = "https://cart.jd.com/gate.action?pid=100016514918&pcount=6&ptype=1";
    pub const ADD_TO_CART_URL: &'static str = "https://cart.jd.com/gate.action?ptype=1";
    pub const UNCHECK_ALL_CART_GOODS_URL: &'static str = "https://api.m.jd.com/api?functionId=pcCart_jc_cartUnCheckAll&appid=JDC_mall_cart&loginType=3";
    pub const COUNTS: usize = 2;

    pub async fn list_cart_goods(
        code: String,
        key_word: String,
        page_no: i64,
    ) -> Result<Option<ShoppingCartPageState>> {
        let addr_prefix = CONFIG.read().await.server_addr();
        let url = format!(
            "{}/api/shopping_cart?page_no={}&key_word={}",
            addr_prefix.as_str(),
            page_no,
            key_word
        );
        let resp = reqwest::Client::new()
            .get(url)
            .header("token", code.as_str())
            .send()
            .await?;
        if resp.status() != StatusCode::OK {
            error!("code:{}, {}", code, resp.status().as_str());
            return Ok(None);
        }
        let mut records: ShoppingCartPageState = serde_json::from_value(resp.json().await?)?;
        // 对于预约商品， 需要更新其预约时间
        for record in records.records.iter_mut() {
            if record.purchase_type.eq(YUYUE) {
                if record.yuyue_start_dt.is_none() {
                    match Self::get_yuyue_goods_infos(record.id, record.sku.clone(), None).await {
                        Ok(info) => {
                            record.yuyue_start_dt = info.yuyue_start_dt;
                            record.yuyue_end_dt = info.yuyue_end_dt;
                            record.yuyue_dt = info.qiang_start_dt;
                            if record.yuyue_start_dt.is_some()
                                || record.yuyue_end_dt.is_some()
                                || record.yuyue_dt.is_some()
                            {
                                // 更新商品预约数据
                                let code = code.clone();
                                let body = json!({
                                    "op":4,
                                    "yuyue_start_dt":datetime_fmt_option(&record.yuyue_start_dt),
                                    "yuyue_end_dt":datetime_fmt_option(&record.yuyue_end_dt),
                                    "yuyue_dt":datetime_fmt_option(&record.yuyue_dt,)
                                })
                                .to_string();
                                info!("yuyue_info:{}-{}:{}", record.id, record.sku, body);
                                Self::update_yuyue_cart_goods(code, record.id.clone(), body)
                                    .await?;
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Failed to get yuyue info:{}-{}, {:?}",
                                record.id, record.sku, e
                            );
                        }
                    }
                } else {
                    debug!("Ignore to update yuyue dt, {}-{}", record.id, record.sku);
                }
            }
        }
        Ok(Some(records))
    }

    pub async fn delete_cart_goods(code: String, ids: Option<Vec<IDType>>) -> Result<()> {
        let addr_prefix = CONFIG.read().await.server_addr();
        let url = format!("{}/api/shopping_cart", addr_prefix.as_str());
        let body;
        if let Some(ids) = ids {
            body = json!({ "ids": ids })
        } else {
            // 删除全部
            body = json!({});
        }
        let resp = reqwest::Client::new()
            .delete(url)
            .header("token", code.as_str())
            .body(body.to_string())
            .send()
            .await?;
        if resp.status() != StatusCode::OK {
            error!("code:{}, {}", code, resp.status().as_str());
        }
        Ok(())
    }

    pub async fn create_cart_goods(code: String, body: String) -> Result<()> {
        let addr_prefix = CONFIG.read().await.server_addr();
        info!("{}", body);
        let url = format!("{}/api/shopping_cart", addr_prefix.as_str());
        let resp = reqwest::Client::new()
            .post(url)
            .header("token", code.as_str())
            .body(body)
            .send()
            .await?;
        if resp.status() != StatusCode::OK {
            error!("code:{}, {}", code, resp.status().as_str());
        }
        Ok(())
    }

    pub async fn update_cart_goods(code: String, id: IDType, body: String) -> Result<IDType> {
        let addr_prefix = CONFIG.read().await.server_addr();
        let url = format!("{}/api/shopping_cart/{}", addr_prefix.as_str(), id);
        let resp = reqwest::Client::new()
            .put(url)
            .header("token", code.as_str())
            .body(body)
            .send()
            .await?;
        if resp.status() != StatusCode::OK {
            error!("code:{}, {}", code, resp.status().as_str());
        }
        Ok(id)
    }
    pub async fn update_yuyue_cart_goods(code: String, id: IDType, body: String) -> Result<IDType> {
        let addr_prefix = CONFIG.read().await.server_addr();
        let url = format!("{}/api/shopping_cart/yuyue/{}", addr_prefix.as_str(), id);
        let resp = reqwest::Client::new()
            .put(url)
            .header("token", code.as_str())
            .body(body)
            .send()
            .await?;
        if resp.status() != StatusCode::OK {
            error!("code:{}, {}", code, resp.status().as_str());
        }
        Ok(id)
    }

    pub async fn update_cart_goods_purchase_link(code: String, body: String) -> Result<()> {
        let addr_prefix = CONFIG.read().await.server_addr();
        let url = format!("{}/api/shopping_cart", addr_prefix.as_str());
        let resp = reqwest::Client::new()
            .put(url)
            .header("token", code.as_str())
            .body(body)
            .send()
            .await?;
        if resp.status() != StatusCode::OK {
            error!("code:{}, {}", code, resp.status().as_str());
        }
        Ok(())
    }

    pub async fn delete_one_cart_goods(code: String, id: IDType) -> Result<()> {
        let addr_prefix = CONFIG.read().await.server_addr();
        let url = format!("{}/api/shopping_cart/{}", addr_prefix.as_str(), id);
        let resp = reqwest::Client::new()
            .delete(url)
            .header("token", code.as_str())
            .send()
            .await?;
        if resp.status() != StatusCode::OK {
            error!("code:{}, {}", code, resp.status().as_str());
        }
        Ok(())
    }

    async fn uncheck_all_cart_goods(account_id: IDType, client: reqwest::Client) -> Result<()> {
        let resp = client
            .post(Self::UNCHECK_ALL_CART_GOODS_URL)
            .header("authority", "api.m.jd.com")
            .header("sec-ch-ua", r##"" Not A;Brand";v="99", "Chromium";v="90", "Google Chrome";v="90""##)
            .header("accept", "application/json, text/plain, */*")
            .header("dnt", "1")
            .header("sec-ch-ua-mobile", "?0")
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/90.0.4430.93 Safari/537.36")
            .header("origin", "https://cart.jd.com")
            .header("sec-fetch-site", "same-site")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-dest", "empty")
            .header("referer", "https://cart.jd.com/")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .send()
            .await?;
        let status = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        if status == StatusCode::OK {
            let data: Value = from_str(&txt)?;
            if data["success"].as_bool().unwrap_or(false) {
                info!("取消全选成功:{}", account_id);
                return Ok(());
            }
        }
        error!(
            "账户:{},uncheck_all选购物车失败:{},{}",
            account_id, status, txt
        );
        Err(OpError::UncheckCartGoods(account_id).into())
    }

    async fn select_cart_good(
        account_id: IDType,
        client: reqwest::Client,
        sku: String,
        sku_uuid: String,
        area: String,
    ) -> Result<()> {
        let encoded = form_urlencoded::Serializer::new(String::new())
            .append_pair("functionId", "pcCart_jc_cartCheckSingle")
            .append_pair("appid", "JDC_mall_cart")
            .append_pair(
                "body",
                json!({
                    "operations": [
                        {
                            "carttype": "5",
                            "ThePacks": [
                                {

                                    "sType": 13,
                                    "Id": "",
                                    "TheSkus": [
                                        {

                                            "Id": &sku,
                                            "skuUuid": &sku_uuid,
                                            "useUuid": false
                                        }
                                    ]
                                }
                            ]
                        }
                    ],
                    "serInfo": {
                        "area": &area,

                    }
                })
                .to_string()
                .as_str(),
            )
            .finish();
        let resp = client
            .post("https://api.m.jd.com/api")
            .header("authority", "api.m.jd.com")
            .header("accept", "application/json, text/plain, */*")
            .header("origin", "https://cart.jd.com")
            .header("referer", "https://cart.jd.com/")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(encoded)
            .send()
            .await?;
        let status = resp.status();
        if status == StatusCode::OK {
            let data: Value = resp.json().await?;
            if data["success"].as_bool().unwrap_or(false) {
                info!("账户{}:选择 {} 成功", account_id, sku);
                return Ok(());
            }
        }
        Err(OpError::SelectCartGoods(account_id, sku).into())
    }

    async fn inner_add_cart_goods(
        account_id: IDType,
        client: reqwest::Client,
        num: u32,
        sku: String,
    ) -> Result<()> {
        // // pub const ADD_TO_CART_URL: &'static str = "https://cart.jd.com/gate.action?pid=100016514918&pcount=6&ptype=1";
        // let url = format!("{}&pid={}&pcount={}", Self::ADD_TO_CART_URL, sku, num);
        let purchase_url = format!(
            "https://cart.jd.com/gate.action?pid={}&pcount={}&ptype=1",
            &sku, num
        );
        // 此接口会302的状态码，此时不需要跟踪这个重定向的url
        let resp = client
            .get(purchase_url.as_str())
            .header("authority", "cart.jd.com")
            .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9")
            .header("referer", "https://item.jd.com/")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .send()
            .await?;
        let status = resp.status();
        if status != StatusCode::FOUND && status != StatusCode::OK {
            error!(
                "加入购物车失败:account_id:{}:{},{}",
                account_id,
                status,
                purchase_url.as_str(),
            );
            return Err(OpError::AddCartGoods(account_id, sku.clone()).into());
        }
        info!(
            "加入购物车成功:{}-{},status:{}",
            account_id,
            sku.as_str(),
            status
        );
        Ok(())
    }

    async fn remove_cart_goods(
        client: reqwest::Client,
        sku: String,
        sku_uuid: String,
    ) -> Result<bool> {
        // 移除购物车里的商品
        let url = "https://api.m.jd.com/api";
        let encoded = form_urlencoded::Serializer::new(String::new())
            .append_pair("functionId", "pcCart_jc_cartRemove")
            .append_pair("appid", "JDC_mall_cart")
            .append_pair(
                "body",
                json!({
                    "operations":[{
                        "TheSkus":[{
                            "id":sku.as_str(),
                            "skuUuid":sku_uuid.as_str(),
                            "useUuid":false,
                        }]
                    }]
                })
                .to_string()
                .as_str(),
            )
            .finish();
        let resp = client
            .post(url)
            .header("authority", "api.m.jd.com")
            .header("sec-ch-ua", r##"" Not A;Brand";v="99", "Chromium";v="90", "Google Chrome";v="90""##)
            .header("accept", "application/json, text/plain, */*")
            .header("dnt", "1")
            .header("sec-ch-ua-mobile", "?0")
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/90.0.4430.93 Safari/537.36")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("origin", "https://cart.jd.com")
            .header("sec-fetch-site", "same-site")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-dest", "empty")
            .header("referer", "https://cart.jd.com/")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .body(encoded)
            .send()
            .await?;
        let status = resp.status();
        if status != StatusCode::OK {
            error!("RemoveCartGoods_1, url:{}, {}", url, status);
            Err(OpError::RemoveCartGoods.into())
        } else {
            let v: Value = resp.json().await?;
            let success = v["success"].as_bool().unwrap_or(false);
            info!(
                "RemoveCartGoods_2: {}-{}-{}",
                sku, v["success"], v["message"]
            );
            // info!("RemoveCartGoods: {}-{}", sku, v.to_string());
            Ok(success)
        }
    }

    async fn get_skuuuid_from_cart(client: reqwest::Client, sku: String) -> Result<String> {
        // 主要是拿 skuUuid ， 然后用其去从购物车删除 商品
        let url = "https://api.m.jd.com/api?functionId=pcCart_jc_getCurrentCart&appid=JDC_mall_cart&loginType=3";
        let resp = client
            .post(url)
            .header("authority", "api.m.jd.com")
            .header("sec-ch-ua", r##"" Not A;Brand";v="99", "Chromium";v="90", "Google Chrome";v="90""##)
            .header("accept", "application/json, text/plain, */*")
            .header("dnt", "1")
            .header("sec-ch-ua-mobile", "?0")
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/90.0.4430.93 Safari/537.36")
            .header("origin", "https://cart.jd.com")
            .header("sec-fetch-site", "same-site")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-dest", "empty")
            .header("referer", "https://cart.jd.com/")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .send()
            .await?;
        let status = resp.status();
        if status != StatusCode::OK {
            return Err(OpError::GetCartInfo.into());
        }
        let v: Value = resp.json().await?;
        let success = v["success"].as_bool().unwrap_or(false);
        if !success {
            error!("get_skuuuid_from_cart: {}", v["message"].to_string());
            return Err(OpError::GetCartInfo.into());
        }
        let default = vec![];
        let default_obj = json!({});
        let vendors = v["resultData"]["cartInfo"]["vendors"]
            .as_array()
            .unwrap_or(&default);
        for item in vendors {
            let sorted = item["sorted"].as_array().unwrap_or(&default);
            for iter in sorted {
                let item = &iter["item"];
                debug!("item: {}", serde_json::to_string(item).unwrap_or_default());
                let id = item["Id"]
                    .as_str()
                    .unwrap_or("")
                    .trim_matches(|c| c == '\"')
                    .to_string();
                if id == sku {
                    let skuid = item["skuUuid"]
                        .to_string()
                        .trim_matches(|c| c == '\"')
                        .to_string();
                    debug!("sku in sorted:{}-{}", sku, skuid.as_str());
                    return Ok(skuid);
                }
                let items = item["items"].as_array().unwrap_or(&default);
                debug!("items:{}", serde_json::to_string(items).unwrap_or_default());
                for it in items {
                    let obj = it.get("item").unwrap_or(&default_obj);
                    debug!(
                        "sku:{}, {}",
                        sku.as_str(),
                        serde_json::to_string(obj).unwrap_or_default()
                    );
                    let id = obj["Id"]
                        .as_str()
                        .unwrap_or("")
                        .trim_matches(|c| c == '\"')
                        .to_string();
                    if id == sku {
                        let skuid = obj["skuUuid"]
                            .to_string()
                            .trim_matches(|c| c == '\"')
                            .to_string();
                        info!("sku in item:{}-{}", sku, skuid.as_str());
                        return Ok(skuid);
                    }
                }
            }
        }
        Ok("".to_string())
    }

    async fn set_account_sku_uuid(account_id: IDType, sku: String, sku_uuid: String) -> bool {
        if !sku.is_empty() {
            let mut guard = ACCOUNT_CART_SKU_UUIDS.write().await;
            let entry = guard.entry(account_id).or_default();
            entry.insert(sku, sku_uuid);
            return true;
        }
        false
    }

    pub async fn add_to_cat_and_get_sku_uuid_from_account_cart(
        num: u32,
        sku: Option<String>,
        account_cookies: Vec<(IDType, Arc<String>)>,
    ) -> Result<()> {
        // 主要是拿 skuUuid ， 然后用其去从购物车删除 商品

        let url = "https://api.m.jd.com/api?functionId=pcCart_jc_getCurrentCart&appid=JDC_mall_cart&loginType=3";
        for (account_id, cookie) in account_cookies {
            let client = default_client(cookie.as_str()).build()?;
            if let Some(sku) = &sku {
                if let Err(_) =
                    Self::inner_add_cart_goods(account_id, client.clone(), num, sku.clone()).await
                {
                    // 忽略错误
                }
            }
            let resp = client
                .post(url)
                .header("authority", "api.m.jd.com")
                .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/90.0.4430.93 Safari/537.36")
                .header("origin", "https://cart.jd.com")
                .header("referer", "https://cart.jd.com/")
                .header("accept-language", "zh-CN,zh;q=0.9")
                .send()
                .await?;
            let status = resp.status();
            if status != StatusCode::OK {
                error!(
                    "获取用户购物车失败:{}, {},{}",
                    account_id,
                    status,
                    resp.text().await.unwrap_or_default()
                );
                continue;
            }
            let v: Value = resp.json().await?;
            let success = v["success"].as_bool().unwrap_or(false);
            if !success {
                error!("获取用户购物车失败, get_skuuuid_from_cart:{}", account_id);
                continue;
            }
            let default = vec![];
            let default_obj = json!({});
            let vendors = v["resultData"]["cartInfo"]["vendors"]
                .as_array()
                .unwrap_or(&default);
            for item in vendors {
                let sorted = item["sorted"].as_array().unwrap_or(&default);
                for iter in sorted {
                    let item = &iter["item"];
                    debug!("item: {}", serde_json::to_string(item).unwrap_or_default());
                    let id = item["Id"]
                        .as_str()
                        .unwrap_or("")
                        .trim_matches(|c| c == '\"')
                        .to_string();
                    let skuid = item["skuUuid"]
                        .to_string()
                        .trim()
                        .trim_matches(|c| c == '\"')
                        .to_string();
                    debug!("sku in sorted:{}-{}", id, skuid.as_str());
                    if Self::set_account_sku_uuid(account_id, id, skuid).await {
                        // 找到一个商品的skuUuid
                        break;
                    }
                    let items = item["items"].as_array().unwrap_or(&default);
                    debug!("items:{}", serde_json::to_string(items).unwrap_or_default());
                    for it in items {
                        let obj = it.get("item").unwrap_or(&default_obj);

                        let id = obj["Id"]
                            .as_str()
                            .unwrap_or("")
                            .trim_matches(|c| c == '\"')
                            .to_string();
                        let skuid = obj["skuUuid"]
                            .to_string()
                            .trim()
                            .trim_matches(|c| c == '\"')
                            .to_string();
                        debug!(
                            "{}-{}-{},{}",
                            account_id,
                            id,
                            skuid,
                            serde_json::to_string(obj).unwrap_or_default()
                        );
                        if Self::set_account_sku_uuid(account_id, id, skuid).await {
                            // 找到一个商品的skuUuid
                            break;
                        }
                    }
                }
            }
        }
        debug!(
            "购物车skuUuid信息:{}",
            to_string(&*ACCOUNT_CART_SKU_UUIDS.read().await).unwrap_or_default()
        );
        Ok(())
    }

    // fn read_cookie(name: &str, cookies_map: &HashMap<String, String>) -> String {
    fn read_cookie(name: &str, cookies_map: &HashMap<&str, &str>) -> String {
        if let Some(v) = cookies_map.get(name) {
            return v.to_string();
        }
        return "".to_string();
    }

    async fn get_goods_info(
        client: reqwest::Client,
        sku: String,
        account_id: IDType,
    ) -> Result<Option<PInfo>> {
        let prod_info_url = format!("https://item.m.jd.com/product/{}.html", sku.as_str());
        let resp = client.get(prod_info_url).send().await?;
        let status = resp.status();
        if status == StatusCode::OK {
            let txt = resp.text().await?;
            // 格式： "item": {"dataFrom":1
            let pt1 = r###""item":"###;
            if let Some(idx1) = txt.find(pt1) {
                let txt = &txt.as_str()[idx1 + pt1.len()..];
                let pt2 = "});";
                if let Some(idx2) = txt.find(pt2) {
                    let pinfo = txt[..idx2].trim();
                    debug!("pinfo:{}-{}-{}", account_id, sku, pinfo);
                    let info = from_str::<PInfo>(pinfo)?;
                    return Ok(Some(info));
                }
            }
        }
        return Ok(None);
    }

    pub async fn get_and_receive_coupons(
        cookie: Arc<String>,
        sku: String,
        account_id: IDType,
    ) -> Result<(IDType, String)> {
        // 领取优惠券
        let client = default_client(cookie.as_str())
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let info = Self::get_goods_info(client.clone(), sku.clone(), account_id).await?;
        if info.is_none() {
            warn!("不能获取商品优惠券信息:{}-{}", account_id, sku);
            return Ok((account_id, sku));
        }
        let info = info.unwrap();
        let list_coupons = format!(
            "https://cd.jd.com/coupon/service?skuId={}&cat={}&venderId={}",
            sku.as_str(),
            info.category.join(","),
            info.venderID.as_str()
        );
        // 查找该商品的优惠券
        let resp = client
            .get(list_coupons)
            .header(
                "referer",
                format!("https://item.jd.com/{}.html", sku.as_str()),
            )
            .header("accept-language", "zh-CN,zh;q=0.9")
            .send()
            .await?;
        let data;
        let status = resp.status();
        let _t = resp.text().await?;
        match from_str::<Value>(_t.as_str()) {
            Ok(v) => {
                data = v;
            }
            Err(e) => {
                error!("resp.json:{}, {}, {:?}", status, _t, e);
                return Err(e.into());
            }
        }
        let v = vec![json!({})];
        let skuConpons = data["skuConpons"].as_array().unwrap_or(&v);
        let currentSkuConpons = data["currentSkuConpons"].as_array().unwrap_or(&v);
        // 自动领完所有优惠券
        let mut coupons = HashMap::new();
        // 先归类， 然后再统一查询同优惠券名对应的优惠券
        for item in skuConpons.iter().chain(currentSkuConpons.iter()) {
            if item["roleId"].is_null() || item["name"].is_null() {
                continue;
            }
            // 优惠券信息的数据结构
            // {"area":1,"batchId":786080894,"couponKind":1,"couponType":1,"discount":30,"key":"g4ueiadcec2e062f4ff7b095a4bd7bd9","name":"仅可购买618头号京贴活动商品","quota":200,"roleId":51718093,"toUrl":"www.jd.com,m.jd.com","url":"//coupon.jd.com/ilink/couponActiveFront/linkKey/front_index.action?linkKey=AAROH_xIpeffAs_-naABEFoeiRjkeX7ip4V8iL_SNqoHv0XA7ndgh62S8FhvWwSUJWiIzciJ94N51uQFtNWAwLf555NkDA&to=www.jd.com","userClass":10000,"addDays":0,"beginTime":"2021-06-01 00:00","endTime":"2021-06-03 23:59","hourCoupon":2,"timeDesc":"有效期2021-06-01 00:00至2021-06-03 23:59","limitType":5,"overlap":2,"discountDesc":"{\"high\":\"40000\",\"info\":[{\"quota\":\"200\",\"discount\":\"30\"}]}","couponStyle":28,"discountFlag":0,"overlapDesc":"可与限品类东券、店铺东券叠加","batchBusinessLabel":"204","topSubsidy":true}
            let id = item["roleId"].to_string().trim().to_string();
            // info!("itemmmmmm1:{}", item.to_string());
            let name = item["name"].as_str().unwrap_or_default().trim().to_string();
            if id.is_empty() || name.is_empty() {
                continue;
            }
            let mut ids = coupons.entry(name).or_insert(HashSet::new());
            ids.insert(id);
        }
        debug!("coupons: {}", to_string(&coupons).unwrap_or_default());
        let mut to_be_get_coupons = vec![];
        for (name, ids) in coupons.into_iter() {
            // 在领取中心搜索 name 的 并在其中找到 batch_id 对应的商品优惠券
            let search_url = format!("https://a.jd.com/search.html?searchText={}", &name);
            let resp = client
                .get(&search_url)
                .header("Referer", "https://a.jd.com/")
                .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9")
                .header("Accept-Language", "zh-CN,zh;q=0.9")
                .send()
                .await?;
            let status = resp.status();
            let txt = resp.text().await?;
            if status != StatusCode::OK {
                warn!("查询优惠券信息时失败:{}-{}, {}", search_url, status, txt);
                continue;
            }
            let document = Html::parse_fragment(&txt);
            let trims: &[_] = &['"', '\''];
            for batchId in ids {
                // let div_id = format!("'#{}'", &batchId);
                // info!("div_id:[{}]", div_id);
                // let sel = Selector::parse(div_id.as_str()).unwrap();
                let sel = Selector::parse("div.quan-d-item").unwrap();
                for element in document.select(&sel) {
                    if let Some(id) = element.value().attr("id") {
                        let id = id.trim();
                        if !id.eq(&batchId) {
                            continue;
                        }
                    } else {
                        warn!("没有 id 属性，略过! {:?}", element);
                        continue;
                    }
                    if let Some(key) = element.value().attr("data-key") {
                        let key = key.trim_matches(trims);
                        to_be_get_coupons.push((
                            batchId.clone(),
                            name.clone(),
                            search_url.clone(),
                            format!("https://a.jd.com/ajax/freeGetCoupon.html?key={}", key),
                        ));
                    }
                }
            }
        }
        info!(
            "to_be_get_coupons: {}",
            to_string(&to_be_get_coupons).unwrap_or_default()
        );

        for (batch_id, name, search_url, url) in to_be_get_coupons.into_iter() {
            let resp1 = client
                .get(&url)
                .header("Referer", &search_url)
                .header("X-Requested-With", "XMLHttpRequest")
                .header("Accept", "application/json, text/javascript, */*; q=0.01")
                .send()
                .await?;
            let status = resp1.status();
            let txt1 = resp1.text().await?;
            if status != StatusCode::OK {
                warn!("领取优惠券时失败:{}-{}, {}", search_url, status, txt1);
                continue;
            }
            // 领取成功时返回的信息： {"value":999,"desc":"领取成功！感谢您的参与，祝您购物愉快~"}
            if txt1.contains("999") {
                info!(
                    "领取优惠券成功:{}-{}, {}-{}:{}",
                    account_id, sku, batch_id, name, txt1
                );
            } else {
                warn!(
                    "领取优惠券失败:{}-{}, {}-{}:{}",
                    account_id, sku, batch_id, name, txt1
                );
            }
        }
        Ok((account_id, sku))
    }

    async fn summit_cart_goods(
        client: reqwest::Client,
        sku: String,
        account_id: IDType,
    ) -> Result<(bool, String)> {
        // 提交订单前需要生成订单信息
        let order_url = "https://trade.jd.com/shopping/order/getOrderInfo.action";
        let resp = client
            .get(order_url)
            .header("sec-ch-ua", r##"" Not A;Brand";v="99", "Chromium";v="90", "Google Chrome";v="90""##)
            .header("sec-ch-ua-mobile", "?0")
            .header("upgrade-insecure-requests", "1")
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/90.0.4430.93 Safari/537.36")
            .header("sec-fetch-site", "same-site")
            .header("sec-fetch-mode", "navigate")
            .header("sec-fetch-user", "?1")
            .header("sec-fetch-dest", "document")
            .header("authority", "trade.jd.com")
            .header("dnt", "1")
            .header("referer", "https://cart.jd.com/")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .send()
            .await?;
        let status = resp.status();
        if status != StatusCode::OK && status != StatusCode::FOUND {
            error!("get order.info error, order_url:{}, {}", order_url, status);
            return Err(OpError::CreateOrderInfo.into());
        }
        let url = "https://trade.jd.com/shopping/order/submitOrder.action?&presaleStockSign=1";
        let encoded = form_urlencoded::Serializer::new(String::new())
            .append_pair("overseaPurchaseCookies", "")
            .append_pair("submitOrderParam.btSupport", "1")
            .append_pair("submitOrderParam.ignorePriceChange", "0")
            .append_pair("submitOrderParam.sopNotPutInvoice", "false")
            .append_pair("submitOrderParam.trackID", "TestTrackId")
            .append_pair("submitOrderParam.payPassword", "")
            .append_pair("submitOrderParam.isBestCoupon", "1")
            .append_pair("submitOrderParam.jxj", "1")
            .append_pair("presaleStockSign", "1")
            .append_pair("vendorRemarks", "[]")
            .finish();
        let req_builder = client
            .post(url)
            .header(
                "sec-ch-ua",
                r#""Google Chrome";v="89", "Chromium";v="89", ";secNot A Brand";v="99""#,
            )
            .header("dnt", "1")
            .header("sec-ch-ua-mobile", "?0")
            .header("user-agent", get_useragent())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("accept", "application/json, text/javascript, */*; q=0.01")
            .header(
                "referer",
                "https://trade.jd.com/shopping/order/getOrderInfo.action",
            )
            .header("X-Requested-With", "XMLHttpRequest")
            .body(encoded);
        let resp = req_builder.send().await?;
        if resp.status() != StatusCode::OK {
            error!("url:{}, {}", url, resp.status().as_str());
            Err(OpError::SubmitOrder.into())
        } else {
            let v: Value = resp.json().await?;
            let success = v["success"].as_bool().unwrap_or(false);
            if success {
                info!(
                    "下单成功，账号:{}, sku:{}, 单号:{}！请前往东京官方商城付款!",
                    account_id, sku, v["orderId"]
                );
            } else {
                warn!(
                    "下单失败，账号:{}-{},{},{}",
                    account_id, sku, v["resultCode"], v["message"]
                );
            }
            Ok((success, v["message"].to_string()))
        }
    }

    async fn get_prod_sku_uuid(account_id: IDType, sku: &str) -> String {
        let mut skuUuid = String::new();
        if let Some(user_map) = ACCOUNT_CART_SKU_UUIDS.read().await.get(&account_id) {
            if let Some(_sku_uuid) = user_map.get(sku) {
                skuUuid = _sku_uuid.clone();
            }
        };
        skuUuid
    }

    pub async fn submit_order_wrapper(
        account_id: IDType,
        cookie: Arc<String>,
        cart_goods_id: IDType,
        sku: String,
        num: u32,
        area: String,
        is_add_to_cart: bool,
        _workers: usize,
    ) -> Result<(IDType, u32, &'static str)> {
        let lock = get_account_cart_lock(account_id).await;
        let buy_success = Arc::new(Mutex::new(false));
        let mut handles = vec![];
        for _ in 1..=_workers {
            let ck = cookie.clone();
            let _sku = sku.clone();
            let _area = area.clone();
            let _lock = lock.clone();
            let _buy_success = buy_success.clone();
            handles.push(async move {
                match Self::submit_order(
                    account_id,
                    ck,
                    cart_goods_id,
                    _sku.clone(),
                    num,
                    _area,
                    is_add_to_cart,
                    _lock,
                    _buy_success,
                )
                    .await
                {
                    Ok((_, _, s)) => s == "success",
                    Err(e) => {
                        error!(
                            "购买商品失败:{}-{}-{}:{:?}",
                            account_id, cart_goods_id, _sku, e
                        );
                        false
                    }
                }
            });
        }
        let success = futures::future::join_all(handles).map(|r| r.contains(&true));
        Ok((
            cart_goods_id,
            num,
            if success.await { "success" } else { "fail" },
        ))
    }

    async fn submit_order(
        account_id: IDType,
        cookie: Arc<String>,
        cart_goods_id: IDType,
        sku: String,
        num: u32,
        area: String,
        is_add_to_cart: bool,
        user_cart_lock: AccountCartLockType,
        buy_success: AccountCartBuyResultLockType,
    ) -> Result<(IDType, u32, &'static str)> {
        // 用户购物车排它锁
        let mut _g = user_cart_lock.lock().await;
        let mut guard = buy_success.lock().await;
        if *guard {
            info!(
                "已经成功购买商品，不用重复购买了:{}-{}-{}-{}!",
                account_id, cart_goods_id, sku, num
            );
            return Ok((cart_goods_id, num, "success"));
        }
        info!("开始购买商品:{}-{}-{}", account_id, cart_goods_id, sku);
        let mut success = false;

        let client = proxy_client_builder(account_id, &sku, cookie.clone())
            .await
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        // 避免重复查询 skuUuid
        let mut skuUuid = Self::get_prod_sku_uuid(account_id, &sku).await;
        let mut cond = false;
        'shopping: for i in 1..=Self::COUNTS {
            if !cond {
                // 避免重复反全选、加入购物车、选中操作
                if let Err(e) = Self::uncheck_all_cart_goods(account_id, client.clone()).await {
                    error!("uncheck_all_cart_goods, times[{}/{}]:{}-{}-{}, {:?}", i, Self::COUNTS, account_id, cart_goods_id, sku, e);
                    continue 'shopping;
                }
                if is_add_to_cart || skuUuid.is_empty() {
                    if let Err(e) = Self::add_to_cat_and_get_sku_uuid_from_account_cart(
                        num,
                        Some(sku.clone()),
                        vec![(account_id, cookie.clone())],
                    )
                        .await
                    {
                        error!("加入购物车失败, times[{}/{}]:{}-{}-{}, {:?}", i, Self::COUNTS, account_id, cart_goods_id, sku, e);
                        continue 'shopping;
                    }
                    skuUuid = Self::get_prod_sku_uuid(account_id, &sku).await;
                } else {
                    if let Err(e) = Self::select_cart_good(
                        account_id,
                        client.clone(),
                        sku.clone(),
                        skuUuid.clone(),
                        area.clone(),
                    )
                        .await
                    {
                        error!("select_cart_good, times[{}/{}]:{}-{}-{}, {:?}", i, Self::COUNTS, account_id, cart_goods_id, sku, e);
                        continue 'shopping;
                    }
                }
                cond = true;
            }
            match Self::summit_cart_goods(client.clone(), sku.clone(), account_id).await {
                Ok((suc, msg)) => {
                    info!(
                        "summit_cart_goods_result:{}-{}-{}-{}",
                        account_id, sku, suc, msg
                    );
                    if suc {
                        success = true;
                        break 'shopping;
                    } else {
                        // 优化订单提交次数过快
                        // 60017  您多次提交过快，请稍后再试"
                        // 60040   购买超过限制
                        // 600158  无货
                        // 0       您选择的收货地址不支持当前的配送方式，请重新选择配送方式！
                        if msg.contains("您多次提交过快，请稍后再试") {
                            sleep(1000).await;
                        } else if msg.contains("超过限制") {
                            sleep(200).await;
                        } else if msg.contains("无货")
                            || msg.contains(
                            "您选择的收货地址不支持当前的配送方式，请重新选择配送方式",
                        )
                            || msg.contains("无法覆盖您选择的收货地址")
                        {
                            // 无货, 或者地址设置有问题就不用重试了
                            success = false;
                            break 'shopping;
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "summit_cart_goods , times[{}/{}]:{}-{}-{}, {:?}", i, Self::COUNTS, account_id, cart_goods_id, sku, e);
                    sleep(500).await;
                }
            }
        }
        *guard = success;
        if success {
            Ok((cart_goods_id, num, "success"))
        } else {
            Ok((cart_goods_id, num, "fail"))
        }
    }

    async fn gen_seckill_order_data(
        account_id: IDType,
        client: reqwest::Client,
        sku: String,
        eid: String,
        fp: String,
        num: u32,
    ) -> Result<Value> {
        let resp = client
            .post("https://marathon.jd.com/seckillnew/orderService/pc/init.action")
            .header("Host", "marathon.jd.com")
            .form(&json!({
                "sku":sku.as_str(),
                "num":num,
                "isModifyAddress": false,
            }))
            .send()
            .await?;
        let status = resp.status();
        let txt = resp.text().await?;
        let init_info = parse_json(txt.as_str());
        info!(
            "账号:{}-{}, status:{},init.action: {}",
            account_id,
            sku.as_str(),
            status,
            init_info.to_string()
        );
        let o = json!({});
        let j_1 = json!(1);
        let j__1 = json!(-1);
        let j_emp = json!("");
        let default_address = &init_info["addressList"][0]; // 默认地址dict
        let invoice_info = &init_info["invoiceInfo"]; //# 默认发票信息dict, 有可能不返回
        let bool_invoice_info = if let Some(m) = invoice_info.as_object() {
            !m.is_empty()
        } else {
            false
        };
        Ok(json!({
            "skuId": sku.as_str(),
            "num": num,
            "addressId": default_address["id"],
            "yuShou": init_info["seckillSkuVO"]["extMap"].get("YuShou").unwrap_or(&json!("0")).as_str().unwrap_or("0") != "0",
            "isModifyAddress": false,
            "name": default_address["name"],
            "provinceId": default_address["provinceId"],
            "cityId": default_address["cityId"],
            "countyId": default_address["countyId"],
            "townId": default_address["townId"],
            "addressDetail": default_address["addressDetail"],
            "mobile": default_address["mobile"],
            "mobileKey": default_address["mobileKey"],
            "email": default_address.get("email").unwrap_or(&j_emp),
            "postCode": "",
            "invoiceTitle": invoice_info.get("invoiceTitle").unwrap_or(&j__1),
            "invoiceCompanyName": "",
            "invoiceContent": invoice_info.get("invoiceContentType").unwrap_or(&j_1),
            "invoiceTaxpayerNO": "",
            "invoiceEmail": "",
            "invoicePhone": invoice_info.get("invoicePhone").unwrap_or(&j_emp),
            "invoicePhoneKey": invoice_info.get("invoicePhoneKey").unwrap_or(&j_emp),
            "invoice": if bool_invoice_info{true} else {false},
            "password": "",
            "codTimeType": 3,
            "paymentType": 4,
            "areaCode": "",
            "overseas": 0,
            "phone": "",
            "eid": eid,
            "fp": fp,
            "token": &init_info["token"],
            "pru": ""
        }))
    }
    async fn request_seckill_url(
        account_id: IDType,
        client: reqwest::Client,
        purchase_url: String,
        sku: String,
    ) -> Result<Option<()>> {
        let resp = client
            .get(purchase_url.as_str())
            .header("Referer", format!("https://item.jd.com/{}.html", sku))
            .send()
            .await?;
        let status = resp.status();
        if resp.status() == StatusCode::OK || resp.status() == StatusCode::FOUND {
            info!(
                "账号:{},请求秒杀地址:{},成功.",
                account_id,
                purchase_url.as_str()
            );
            return Ok(Some(()));
        } else {
            warn!(
                "账号:{},请求秒杀地址:{},失败:{}, \n{}!",
                account_id,
                purchase_url.as_str(),
                status,
                resp.text().await?
            );
            return Ok(None);
        }
    }

    pub async fn submit_seckill_order(
        account_id: IDType,
        cookie: Arc<String>,
        cart_goods_id: IDType,
        sku: String,
        eid: String,
        fp: String,
        // purchase_url: String,
    ) -> Result<(IDType, u32, &'static str)> {
        // 抢购商品的下单流程与普通商品不同，不支持加入购物车，可能需要提前预约，主要执行流程如下：
        // 1. 访问商品的抢购链接
        // 2. 访问抢购订单结算页面（好像可以省略这步，待测试）
        // 3. 提交抢购（秒杀）订单
        let mut i = 1;
        let total = Self::COUNTS;
        while i <= total {
            info!(
                "账号[id-{}],第{}/{}次请求抢购:{}",
                account_id, i, total, sku
            );
            let client = proxy_client_builder(account_id, &sku, cookie.clone())
                .await
                .redirect(reqwest::redirect::Policy::none())
                .build()?;
            let purchase_url =
                GoodsService::get_goods_seckill_link2(sku.clone(), client.clone()).await?;
            if let Ok(Some(())) =
            Self::request_seckill_url(account_id, client.clone(), purchase_url, sku.clone())
                .await
            {
                let body = Self::gen_seckill_order_data(
                    account_id,
                    client.clone(),
                    sku.clone(),
                    eid.clone(),
                    fp.clone(),
                    1,
                )
                .await?;
                let url = format!(
                "https://marathon.jd.com/seckillnew/orderService/pc/submitOrder.action?skuId={}",
                sku.as_str()
            );

                match client.post(url.clone())
                    .header("Host", "marathon.jd.com")
                    .header("Referer", format!("https://marathon.jd.com/seckill/seckill.action?skuId={0}&num={1}&rid={2}",
                                               sku.as_str(), 1, PKLocal::now().timestamp()))
                    .form(&body)
                    .send()
                    .await{
                    Ok(resp) =>{
                        let status = resp.status();
                        let txt = resp.text().await.unwrap_or_default();
                        if status != StatusCode::OK{
                            warn!("失败,账号[id-{}]第{}/{}次抢购{}! {}, {}", account_id,i, total, sku.as_str(), status, txt);
                        }else{
                            let rdata: serde_json::Result<Value>= from_str(txt.as_str());
                            match rdata{
                                Ok(resp_json) => {
                                    let success = resp_json["success"].as_bool().unwrap_or_default();
                                    if success{
                                        let order_id = resp_json["orderId"].to_string();
                                        let total_money = resp_json["totalMoney"].to_string();
                                        let pay_url = format!("https:{}", resp_json["pcUrl"].to_string());
                                        info!("账号[id-{}]抢购成功,,订单号:{}, 总价:{}, 电脑端付款链接:{}",account_id, order_id, total_money, pay_url);
                                        return Ok((cart_goods_id, 1, "success"));
                                    }else{
                                        warn!("账号[id-{}]第{}/{}次抢购 {} 失败,返回信息:{}", account_id,i, total, sku.as_str(), txt);
                                    }
                                }
                                Err(e) =>{
                                    error!("parse_json, 账号[id-{}]第{}/{}次抢购 {} 失败，{:?} 返回信息:{}",
                                           account_id, i, total, sku.as_str(), e, txt);
                                }
                            }
                        }
                    }
                    Err(e) =>{
                        error!("request,账号[id-{}],第{}/{}次抢购 {} 失败，{:?}",account_id, i, total, sku.as_str(), e,);
                    }
                }
                i += 1;
                // 休息
                sleep(500).await;
            }
            warn!(
                "账号[id-{}]共{}次抢购{}失败, 返回不再抢购!",
                account_id,
                total,
                sku.as_str()
            );
        }
        return Ok((cart_goods_id, 1, "fail"));
    }
    pub async fn get_yuyue_goods_infos(
        cart_goods_id: IDType,
        sku: String,
        client: Option<Client>,
    ) -> Result<YuyueInfo> {
        // 获取预约商品相关的时间: 预约开始时间， 预约结束时间
        let url = format!("https://item.m.jd.com/product/{}.html", sku.as_str());
        let _client = if let Some(client_) = client {
            client_
        } else {
            default_client("").build()?
        };
        let resp = _client.get(url.as_str()).send().await?;
        let status = resp.status();
        let txt = resp.text().await?;
        let mut info = YuyueInfo::default();
        info.cart_goods_id = cart_goods_id;
        if status == StatusCode::OK {
            let yuyue = r##""yuyue":"##;
            if let Some(idx) = txt.find(yuyue) {
                let start_idx = idx + yuyue.len();
                let txt = &txt.as_str()[start_idx..];
                if let Some(end_idx) = txt.find('}') {
                    // 在商品信息中找以下到的数据结构
                    // "yuyue":{"type":"1","num":6948,"d":218782,"category":"4","stime":"2021-05-19 00:21:12","etime":"2021-05-23 09:54:59","state":2,"sku":100010104457,"isJ":0,"url":"//yushou.jd.com/toYuyue.action?sku=100010104457&key=d1df2da18dc20f13c30d6ce237137bb8","info":"预约进行中","flag":false,"insertTime":1621516116,"yueStime":"2021-05-19 00:21:12","yueEtime":"2021-05-23 09:54:59","qiangStime":"","qiangEtime":"","plusStime":"","plusEtime":"","plusType":0,"plusD":218782,"isBefore":0,"riskCheck":"","hasAddress":false,"address":"","hidePrice":0,"showPromoPrice":"0","sellWhilePresell":"0"},
                    let sku_json_content = &txt[..=end_idx];
                    info!("{},<{}-{}>, {}", sku, start_idx, end_idx, sku_json_content);
                    let jsonv: Value = from_str(sku_json_content)?;
                    let mut stime = jsonv["yueStime"].as_str().unwrap_or_default();
                    let mut etime = jsonv["yueEtime"].as_str().unwrap_or_default();
                    if stime.is_empty() {
                        stime = jsonv["stime"].as_str().unwrap_or_default();
                    }
                    if etime.is_empty() {
                        etime = jsonv["etime"].as_str().unwrap_or_default();
                    }
                    let yuyue_start_dt = parse_datetime(stime);
                    let yuyue_end_dt = parse_datetime(etime);
                    let mut yuyue_url = jsonv["url"].as_str().unwrap_or_default().to_string();
                    if yuyue_url.starts_with("//") {
                        yuyue_url = format!("https:{}", yuyue_url)
                    }
                    let qiang_start_dt =
                        parse_datetime(jsonv["qiangStime"].as_str().unwrap_or_default());
                    info.yuyue_url = yuyue_url;
                    info.yuyue_start_dt = yuyue_start_dt;
                    info.yuyue_end_dt = yuyue_end_dt;
                    info.qiang_start_dt = qiang_start_dt;
                    return Ok(info);
                } else {
                    warn!("{}: Not found }}", sku);
                }
            }
            warn!("{}:Not found {}", sku, yuyue,);
        }
        warn!("获取预约商品[{}]信息失败:{}, {}", sku, status, txt);
        return Ok(info);
    }

    pub async fn submit_yuyue_order(
        account_id: IDType,
        cookie: Arc<String>,
        cart_goods_id: IDType,
        sku: String,
        num: u32,
        in_yuyue: bool,
        area: String,
    ) -> Result<(IDType, u32, &'static str)> {
        // 预约订单流程： 先预约， 然后再购买（或者抢购）
        if in_yuyue {
            let client = default_client(cookie.as_str())
                .redirect(reqwest::redirect::Policy::none())
                .build()?;
            let info =
                Self::get_yuyue_goods_infos(cart_goods_id, sku.clone(), Some(client.clone()))
                    .await?;
            let resp = client
                .get(&info.yuyue_url)
                .header("referer", "https://item.jd.com/")
                .send()
                .await?;
            let status = resp.status();
            if status != StatusCode::OK {
                return Ok((cart_goods_id, num, "fail"));
            }
            return Ok((cart_goods_id, num, "yuyueing"));
        }
        let goods_info = GoodsService::get_goods_info(&sku).await?;
        let (purchase_type, purchase_url) = {
            let document = Html::parse_document(&goods_info);
            GoodsService::get_purchase_info(&document)?
        };
        if purchase_type.eq(SECOND_KILL) {
            info!(
                "预约抢购商品{}-{}-{}-{}:{}",
                account_id, cart_goods_id, sku, purchase_type, purchase_url
            );
            let mut handles = vec![];
            for _ in 1..=workers() {
                let ck = cookie.clone();
                let _sku = sku.clone();
                handles.push(async move {
                    match Self::submit_seckill_order(
                        account_id,
                        ck,
                        cart_goods_id,
                        _sku,
                        "".to_string(),
                        "".to_string(),
                    )
                    .await
                    {
                        Ok((id, num, status)) => status == "success",
                        Err(e) => {
                            error!("{}-{}:{:?}", account_id, cart_goods_id, e);
                            false
                        }
                    }
                });
            }
            let success = join_all(handles)
                .map(|r| {
                    // 有一个成功那就是购买成功了
                    r.into_iter().filter(|i| *i).count() > 0
                })
                .await;
            if success {
                Ok((cart_goods_id, num, "success"))
            } else {
                Ok((cart_goods_id, num, "fail"))
            }
        } else if purchase_type.eq(PRESALE) {
            info!(
                "预约预售商品{}-{}-{}-{}:{}",
                account_id, cart_goods_id, sku, purchase_type, purchase_url
            );
            let mut handles = vec![];
            for _ in 1..=workers() {
                let ck = cookie.clone();
                let _sku = sku.clone();
                handles.push(async move {
                    match Self::submit_presale_order(
                        account_id,
                        ck,
                        cart_goods_id,
                        _sku,
                        "".to_string(),
                        "".to_string(),
                        num,
                    )
                    .await
                    {
                        Ok((id, num, status)) => status == "success",
                        Err(e) => {
                            error!("{}-{}:{:?}", account_id, cart_goods_id, e);
                            false
                        }
                    }
                });
            }
            let success = join_all(handles)
                .map(|r| r.into_iter().filter(|i| *i).count() > 0)
                .await;
            if success {
                Ok((cart_goods_id, num, "success"))
            } else {
                Ok((cart_goods_id, num, "fail"))
            }
        } else {
            info!(
                "预约普通商品的购买:{}-{}-{}-{}:{}",
                account_id, cart_goods_id, sku, purchase_type, purchase_url
            );
            // 普通的预约商品，就按照加入购物车,然后购买的这种流程来做
            Self::submit_order_wrapper(
                account_id,
                cookie,
                cart_goods_id,
                sku,
                num,
                area,
                false,
                workers(),
            )
                .await
        }
    }

    pub async fn submit_seckill_order_by_webdriver(
        account_id: IDType,
        cookie: Arc<String>,
        cart_goods_id: IDType,
        sku: String,
        purchase_url: String,
    ) -> Result<Option<(IDType, u32, &'static str)>> {
        let mut i = 0;
        info!("driver initing....");
        // let mut driver = init_driver_sync(false);
        let mut driver = init_driver(false).await?;
        info!("driver inited...&..get1: {}", purchase_url.as_str());
        let mut success = false;

        {
            driver.get(purchase_url.clone()).await?;
            info!("driver inited...&..get2: {}", purchase_url.as_str());
            while i < 10 {
                match driver.find_element(By::Css("button.checkout-submit")).await {
                    Ok(submit) => {
                        submit.click().await?;
                        info!("下单成功:{}-{}", account_id, sku.as_str());
                        success = true;
                        break;
                    }
                    Err(WebDriverError::NoSuchWindow(e))
                    | Err(WebDriverError::NoSuchAlert(e))
                    | Err(WebDriverError::NoSuchFrame(e)) => {
                        // | WebDriverError::NoSuchElement(e) => {
                        // 窗口被关闭了
                        error!(
                            " submit order error! account:{},  error:{:?}",
                            account_id, e
                        );
                        driver.quit().await?;
                        return Ok(None);
                    }
                    Err(e) => {
                        warn!("{:?}", e);
                        i += 1;
                        sleep(100).await;
                    }
                }
                // 此处下单失败后， 需要把购物车的该商品删除， 再重新跑一遍流程
            }
            driver.quit().await?;
        }
        if success {
            Ok(Some((cart_goods_id, 1, "success")))
        } else {
            Ok(Some((cart_goods_id, 1, "fail")))
        }
    }

    pub async fn submit_presale_order(
        account_id: IDType,
        cookie: Arc<String>,
        cart_goods_id: IDType,
        sku: String,
        eid: String,
        fp: String,
        num: u32,
    ) -> Result<(IDType, u32, &'static str)> {
        let mut success = false;
        let url = format!(
            "https://cart.jd.com/cart/dynamic/gateForSubFlow.action?wids={}&nums={}&subType=32",
            sku.as_str(),
            num,
        );
        let client = proxy_client_builder(account_id, &sku, cookie.clone())
            .await
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let total = Self::COUNTS;
        for i in 1..=total {
            info!("第{}/{}次购买预售商品:{}-{}", i, total, account_id, sku);
            let resp = client.get(url.as_str()).send().await?;
            let status = resp.status();
            if status == StatusCode::FOUND {
                if let Some(u) = resp.headers().get("location") {
                    let order_info_url = u.to_str().unwrap_or_default();
                    let resp = client.get(order_info_url).send().await?;
                    let status = resp.status();
                    if status == StatusCode::FOUND || status == StatusCode::OK {
                        let url = "https://trade.jd.com/shopping/order/submitOrder.action?";
                        let encoded = form_urlencoded::Serializer::new(String::new())
                            .append_pair("overseaPurchaseCookies", "")
                            .append_pair("vendorRemarks", "[]")
                            .append_pair("submitOrderParam.sopNotPutInvoice", "false")
                            .append_pair("submitOrderParam.presalePayType", "2")
                            .append_pair("submitOrderParam.trackID", "TestTrackId")
                            .append_pair("flowType", "15")
                            .append_pair("preSalePaymentTypeInOptional", "2")
                            .append_pair("submitOrderParam.ignorePriceChange", "0")
                            .append_pair("submitOrderParam.btSupport", "0")
                            .append_pair("submitOrderParam.payType4YuShou", "2")
                            .append_pair("submitOrderParam.jxj", "1")
                            .append_pair("submitOrderParam.eid", eid.as_str())
                            .append_pair("submitOrderParam.fp", fp.as_str())
                            .finish();
                        let req_builder = client
                            .post(url)
                            .header(
                                "sec-ch-ua",
                                r#""Google Chrome";v="89", "Chromium";v="89", ";secNot A Brand";v="99""#,
                            )
                            .header("dnt", "1")
                            .header("sec-ch-ua-mobile", "?0")
                            .header("user-agent", get_useragent())
                            .header("Content-Type", "application/x-www-form-urlencoded")
                            .header("accept", "application/json, text/javascript, */*; q=0.01")
                            // .header("referer", order_info_url.as_str())
                            .header("referer", order_info_url)
                            .header("X-Requested-With", "XMLHttpRequest")
                            .body(encoded);
                        let resp = req_builder.send().await?;
                        if resp.status() != StatusCode::OK {
                            error!("url:{}, {}", url, resp.status().as_str());
                            success = false;
                        } else {
                            let v: Value = resp.json().await?;
                            success = v["success"].as_bool().unwrap_or(false);
                            if success {
                                info!(
                                    "下单成功，账号:{}-{}, 单号：{}, {}！请前往东京官方商城付款!",
                                    account_id, sku, v["orderId"], v["message"]
                                );
                            } else {
                                warn!(
                                    "下单失败，账号:{}-{}-{} {}",
                                    account_id, sku, v["resultCode"], v["message"]
                                );
                            }
                        }
                    } else {
                        warn!(
                            "获取订单信息失败:{}-{}, {},{}",
                            account_id, sku, order_info_url, status
                        );
                    }
                }
            } else {
                warn!(
                    "第{}/{}次购买预售商品失败:{}-{}:{},{}",
                    i,
                    total,
                    account_id,
                    sku,
                    status,
                    resp.text().await.unwrap_or_default()
                );
            }
            if success {
                break;
            }
            sleep(500).await;
        }
        if success {
            Ok((cart_goods_id, num, "success"))
        } else {
            Ok((cart_goods_id, num, "fail"))
        }
    }

    pub async fn submit_presale_order_by_webdriver(
        account_id: IDType,
        cookie: Arc<String>,
        cart_goods_id: IDType,
        eid: String,
        fp: String,
        sku: String,
        num: u32,
    ) -> Result<(IDType, u32, &'static str)> {
        let mut driver = init_miaosha_driver(false).await?;
        let url = format!("https://item.jd.com/{}.html", sku.as_str());
        let url = build_driver_url(url, cookie.as_str());
        info!("111:{}", url);
        driver.get(url.as_str()).await?;
        let mut success = false;
        info!("222:{}", url);
        for _ in 0..10 {
            match driver.find_element(By::Css("a#btn-reservation")).await {
                Ok(submit) => {
                    if let Some(mut link) = submit.get_attribute("href").await? {
                        if link.starts_with("//") {
                            link = format!("https:{}", link)
                        }
                        info!("定金链接:{}-{}-{}", account_id, sku.as_str(), link);
                        // let link = build_driver_url(link, cookie.as_str());
                        // driver.get(link).await?;
                        submit.click().await?;
                        match driver.find_element(By::Id("presaleEarnest")).await {
                            Ok(dingjin_box) => {
                                dingjin_box.click().await?;
                                match driver
                                    .find_element(By::Id("enterPriseUserPaymentSubmit"))
                                    .await
                                {
                                    Ok(submit_button) => {
                                        submit_button.click().await?;
                                        success = true;
                                        break;
                                    }
                                    Err(e) => {
                                        error!(
                                            "找不到提交订单按钮,{}-{}-{}, {:?}",
                                            account_id, cart_goods_id, sku, e
                                        );
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                error!(
                                    "找不到同意支付定金按钮,{}-{}-{}, {:?}",
                                    account_id, cart_goods_id, sku, e
                                );
                                break;
                            }
                        }
                    } else {
                        warn!("下单失败:{}-{}", account_id, sku.as_str());
                        submit.click().await?;
                    }
                }
                Err(WebDriverError::NoSuchWindow(e))
                | Err(WebDriverError::NoSuchAlert(e))
                | Err(WebDriverError::NoSuchFrame(e)) => {
                    // | WebDriverError::NoSuchElement(e) => {
                    // 窗口被关闭了
                    error!(
                        " submit presale order error! account:{}-{}-{},  error:{:?}",
                        account_id, cart_goods_id, sku, e
                    );
                    return Ok((cart_goods_id, num, "fail"));
                }
                Err(e) => {
                    warn!("{:?}", e);
                    sleep(100).await;
                }
            }
        }
        driver.quit().await?;
        if success {
            Ok((cart_goods_id, num, "success"))
        } else {
            Ok((cart_goods_id, num, "fail"))
        }
    }
}
