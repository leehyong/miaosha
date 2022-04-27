use crate::error::{JdMiaoshaError, OpError, Result};
use crate::models::{OrderInfo, ProdPlatform};
use log::{debug, error, info, warn};
use scraper::{Html, Selector};
use std::collections::{BTreeMap};
use std::sync::Arc;
use thirtyfour::{WebDriverCommands};
use url::form_urlencoded;

#[derive(Clone)]
pub enum QueryCondition {
    KeyWord(String),  // 关键字查询
    Unpaid,           // 待付款, 1
    WaitForReceiving, // 待收货, 128
    Finish,           // 已完成， 1024
    All,              // 全部订单， 4096
}
impl Default for QueryCondition {
    fn default() -> Self {
        Self::Unpaid
    }
}

#[derive(Clone)]
pub struct OrderService;

impl OrderService {
    pub async fn get_orders_by_user(
        account: String,
        cookie_str: Arc<String>,
        cond: QueryCondition,
    ) -> Result<BTreeMap<String, OrderInfo>> {
        let url = match cond.clone() {
            QueryCondition::KeyWord(kw) => {
                format!("https://order.jd.com/center/search.action?keyword={}", kw)
            }
            _ => {
                format!(
                    "https://order.jd.com/center/list.action?s={}&d=1",
                    match cond {
                        QueryCondition::Unpaid => "1",
                        QueryCondition::WaitForReceiving => "128",
                        QueryCondition::Finish => "1024",
                        QueryCondition::All => "4096",
                        _ => unreachable!(),
                    }
                )
            }
        };

        let resp = reqwest::Client::new()
            .get(url)
            .header("cookie", cookie_str.as_str())
            .header("authority", "order.jd.com")
            .header("cache-control", "max-age=0")
            .header("sec-ch-ua", r#"" Not A;Brand";v="99", "Chromium";v="90", "Google Chrome";v="90""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("dnt", "1")
            .header("upgrade-insecure-requests", "1")
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/90.0.4430.72 Safari/537.36")
            .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9")
            .header("sec-fetch-site", "same-site")
            .header("sec-fetch-mode", "navigate")
            .header("sec-fetch-user", "?1")
            .header("sec-fetch-dest", "document")
            .header("referer", "https://item.jd.com/")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .send()
            .await?;
        let html_body = resp.text().await?;
        if html_body.contains("我的订单") {
            let document = Html::parse_document(html_body.as_str());
            let one_order_row = Selector::parse("tr.tr-bd").unwrap(); // 找到商品行
            let prod_name = Selector::parse("div.p-name a").unwrap(); // 找到商品名称
            let goods_num = Selector::parse("div.goods-number").unwrap(); // 找到商品数量
            let money_amount = Selector::parse("div.amount span").unwrap(); // 找到商品总价
            let receiver = Selector::parse("div.consignee span").unwrap(); // 找到商品收货人
            let order_status = Selector::parse("span.order-status").unwrap(); // 找到商品状态
            let mut ret = BTreeMap::new();
            for element in document.select(&one_order_row) {
                if let Some(order_no) = element.value().attr("id") {
                    // attr("id")格式->track163821896625
                    let (_, _order_no) = order_no.split_at(5);
                    if ret.get(_order_no).is_some(){
                        // 同一个订单里有多个商品时，只显示第一个商品;
                        // todo: 以后再兼容 一个订单多个商品的情况
                        continue;
                    }
                    let mut order = OrderInfo::default();
                    order.order_no = _order_no.to_string();
                    // 订单生成时间
                    let dt_selector =
                        Selector::parse(format!("input#datasubmit-{}", _order_no).as_str())
                            .unwrap();
                    if let Some(e) = document.select(&dt_selector).next() {
                        order.create_dt = e.value().attr("value").unwrap_or("").trim().to_string();
                    }
                    let inner = Html::parse_fragment(element.inner_html().as_str());
                    if let Some(e) = inner.select(&prod_name).next() {
                        order.name = e.text().next().unwrap_or("").trim().to_string();
                    }
                    if let Some(e) = inner.select(&goods_num).next() {
                        order.purchase_num = e.text().next().unwrap_or("").trim().to_string();
                    }
                    if let Some(e) = inner.select(&money_amount).next() {
                        order.total_price = e.text().next().unwrap_or("").trim().to_string();
                    }
                    if let Some(e) = inner.select(&receiver).next() {
                        order.receiver = e.text().next().unwrap_or("").trim().to_string();
                    }
                    if let Some(e) = inner.select(&order_status).next() {
                        order.status = e.text().next().unwrap_or("").trim().to_string();
                    }
                    order.account = account.clone();
                    // info!("account:{},order:{}", account, serde_json::to_string(&order).unwrap_or_default());
                    order.cookie = cookie_str.clone();
                    ret.insert(order.order_no.clone(), order);
                }
            }
            Ok(ret)
        } else {
            // 用户cookie 过期了
            Err(OpError::CookieExpired(account).into())
        }
    }
}
