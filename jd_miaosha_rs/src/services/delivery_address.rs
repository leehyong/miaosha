use crate::error::{JdMiaoshaError, OpError, Result};
use crate::models::AddressInfo;
use http::{Method, StatusCode};
use log::{debug, error, info, warn};
use rand::prelude::*;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, ClientBuilder, Error as ReqwestError, Request,
};
use scraper::{Html, Selector};
use serde::Deserialize;
use serde_json::{from_slice, from_str, json, to_string, Value};
use std::cell::RefCell;
use std::collections::LinkedList;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use thirtyfour::error::WebDriverError;
use thirtyfour::prelude::*;
use thirtyfour::GenericWebDriver;
use thirtyfour::{By, WebDriverCommands};
use tokio::fs::{create_dir_all, File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::RwLock;
use url::form_urlencoded;

#[derive(Clone)]
pub struct DeliveryAddressService;

impl DeliveryAddressService {
    pub async fn get_all_address_by_user(
        account: String,
        cookie_str: Arc<String>,
    ) -> Result<LinkedList<AddressInfo>> {
        let url = "https://easybuy.jd.com/address/getEasyBuyList.action";
        let resp = reqwest::Client::new()
            .get(url)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9")
            .header("Accept-Language", "zh-CN,zh;q=0.9")
            .header("Cache-Control", "max-age=0")
            .header("Connection", "keep-alive")
            .header("DNT", "1")
            .header("Referer", "https://home.jd.com/")
            .header("Upgrade-Insecure-Requests", "1")
            .header("User-Agent", "User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 11_1_0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.114 Safari/537.36")
            .header("sec-ch-ua", r#""Google Chrome";v="89", "Chromium";v="89", ";Not A Brand";v="99""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("Sec-Fetch-Dest", "document")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", "same-site")
            .header("Sec-Fetch-User", "?1")
            .header("Cookie", cookie_str.as_str())
            .send()
            .await?;
        let resp = resp.text().await?;
        // let body = to_bytes(resp.into_body()).await?;
        // let resp_html = ::std::str::from_utf8(&to_bytes(resp.into_body()).await?).unwrap().to_string();
        // let resp_html = String::from_utf8(body.chunk().to_vec()).unwrap();
        let document = Html::parse_document(resp.as_str());
        let addr_selector = Selector::parse("div#addressList div.easebuy-m").unwrap(); // 找到商品名称
        let left_col_selector = Selector::parse("div.item-lcol").unwrap();
        let item_selector = Selector::parse("div.item").unwrap();
        let label_selector = Selector::parse("span.label").unwrap();
        let fr_selector = Selector::parse("div.fl").unwrap();
        let mut ret = LinkedList::new();
        let mut i = 0;
        for element in document.select(&addr_selector) {
            let mut addr = AddressInfo::default();
            addr.account = account.as_str().to_owned();
            addr.cookie = cookie_str.clone();
            addr.is_latest_receive_addr = false;
            i += 1;
            if let Some(eid) = element.value().attr("id") {
                // eid 的格式 addresssDiv-3759910743
                addr.addr_id = eid.split("-").skip(1).next().unwrap().to_owned();
            }
            if let Some(left_div) = element.select(&left_col_selector).next() {
                for item in left_div.select(&item_selector) {
                    if let Some(label) = item.select(&label_selector).next() {
                        if let Some(mut txt) = label.text().next() {
                            txt = txt.trim();
                            let sibling_txt = item
                                .select(&fr_selector)
                                .next()
                                .unwrap()
                                .text()
                                .next()
                                .unwrap_or("")
                                .trim()
                                .to_owned();
                            match txt {
                                "收货人：" => addr.receiver = sibling_txt,
                                "所在地区：" => addr.area_zone = sibling_txt,
                                "地址：" => addr.address = sibling_txt,
                                "手机：" => addr.mobile_phone = sibling_txt,
                                "固定电话：" => addr.fixed_line_phone = sibling_txt,
                                "电子邮箱：" => addr.email = sibling_txt,
                                _ => {
                                    warn!("Unknown text")
                                }
                            }
                        }
                    } else {
                        error!("error document!")
                    }
                }
            }
            ret.push_back(addr);
        }
        Ok(ret)
    }

    pub async fn set_order_express_address(
        addr_id: String,
        account: String,
        cookie_str: Arc<String>,
    ) -> Result<Option<(String, String)>> {
        // 设置结算时订单的收货地址
        // 京东在展示商品的时候，是按照最近一次订单（不管订单有没有支付或者完成、失败）的收货地址来展示商品的库存的
        let url = "https://trade.jd.com/shopping/dynamic/consignee/saveConsignee.action";
        let encoded = form_urlencoded::Serializer::new(String::new())
            .append_pair("consigneeParam.newId", addr_id.as_str()) // 设置地址id, 通过 get_all_address 获取
            .append_pair("consigneeParam.type", "null")
            .append_pair("consigneeParam.commonConsigneeSize", "0")
            .append_pair("consigneeParam.isUpdateCommonAddress", "0")
            .append_pair("consigneeParam.giftSenderConsigneeName", "")
            .append_pair("consigneeParam.giftSendeConsigneeMobile", "")
            .append_pair("consigneeParam.noteGiftSender", "false")
            .append_pair("consigneeParam.isSelfPick", "0")
            .append_pair("consigneeParam.selfPickOptimize", "0")
            .append_pair("consigneeParam.pickType", "0")
            .append_pair("presaleStockSign", "0") // 这个参数没得什么实际作用， 只需要有就行了
            .finish();
        let resp = reqwest::Client::new().post(url)
            .header("authority", "trade.jd.com")
            .header("sec-ch-ua", r#""Google Chrome";v="89", "Chromium";v="89", ";Not A Brand";v="99""#)
            .header("Accept", "application/json, text/javascript, */*; q=0.01")
            .header("DNT", "1")
            .header("X-Requested-With", "XMLHttpRequest")
            .header("sec-ch-ua-mobile", "?0")
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 11_1_0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.114 Safari/537.36")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Origin", "https://easybuy.jd.com")
            .header("Sec-Fetch-Site", "same-origin")
            .header("Sec-Fetch-Mode", "cors")
            .header("Sec-Fetch-Dest", "empty")
            .header("Referer", "https://trade.jd.com/shopping/order/getOrderInfo.action")
            .header("accept-Language", "zh-CN,zh;q=0.9")
            .header("Cookie", cookie_str.as_str())
            .body(encoded)
            .send()
            .await?;
        let status = resp.status();
        if status == StatusCode::OK {
            info!("{}", resp.text().await?);
            Ok(Some((account, addr_id)))
        }else{
            Ok(None)
        }
    }
}
