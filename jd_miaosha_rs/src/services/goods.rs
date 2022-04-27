use std::collections::BTreeMap;
use std::time::Duration;

use chrono::Datelike;
use futures::{join, select};
use http::{Method, StatusCode};
use log::{debug, error, info, warn};
use rand::prelude::*;
use reqwest::{
    Client,
    ClientBuilder, Error as ReqwestError, header::{HeaderMap, HeaderValue}, Request,
};
use scraper::{Html, Selector};
use serde_json::{from_slice, from_str, json, Value};
use thirtyfour::{By, WebDriverCommands};
use url::form_urlencoded;

use crate::*;
use crate::error::{JdMiaoshaError, OpError, Result};
use crate::models::{GoodsState, StockStatus};
use crate::services::driver::*;
use crate::utils::*;

#[derive(Clone)]
pub struct GoodsService {
    http_client: Client,
}

thread_local! {
    static SERVICE:GoodsService = GoodsService::new();
}

lazy_static! {
    static ref DEFAULT_HEADER: HeaderMap = {
        let mut hm = HeaderMap::new();
        hm.insert("Connection", "keep-alive".parse().unwrap());
        hm.insert("Accept-Language", "zh-CN,zh;q=0.9".parse().unwrap());
        hm.insert("DNT", "1".parse().unwrap());
        hm.insert("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 11_1_0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.114 Safari/537.36".parse().unwrap());
        hm.insert(
            "sec-ch-ua",
            r#""Google Chrome";v="89", "Chromium";v="89", ";Not A Brand";v="99""#
                .parse()
                .unwrap(),
        );
        hm
    };
}

impl GoodsService {
    const STOCK_URL: &'static str = "https://c0.3.cn/stocks";
    const PRICE_URL: &'static str = "http://p.3.cn/prices/mgets";
    // const ADD_TO_SHOPPING_CART_URL: &'static str = "https//cart.jd.com/gate.action?pid={}&pcount={}&ptype=1&gs=2";

    fn new() -> Self {
        let client = ClientBuilder::new()
            .default_headers(DEFAULT_HEADER.clone())
            .connect_timeout(Duration::from_millis(12000))
            .tcp_keepalive(Duration::from_secs(30))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self {
            http_client: client,
        }
    }
    pub async fn get_prod_stock(
        cart_goods_id: IDType,
        sku: String,
        area_id: String,
    ) -> Result<(IDType, StockStatus)> {
        let r = Self::inner_get_prod_stock(sku.as_str(), area_id.as_str()).await?;
        Ok((cart_goods_id, r))
    }

    async fn inner_get_prod_stock(prod_id: &str, area_id: &str) -> Result<StockStatus> {
        let encoded = form_urlencoded::Serializer::new(String::new())
            .append_pair("type", "getstocks")
            .append_pair("skuIds", prod_id)
            .append_pair("area", area_id)
            .finish();
        let resp = reqwest::get(format!("{}?{}", Self::STOCK_URL, encoded)).await?;
        let status = resp.status();
        if status == StatusCode::OK {
            // 只能通过下面的方式，使用两次变化，才能正常使用 --begin
            let v = resp.text().await?;
            let v_json: Value = serde_json::from_str(v.as_str())?;
            // 只能通过上面的方式，使用两次变化，才能正常使用 --end
            let stock = v_json[prod_id]["StockState"].to_string();
            debug!(
                "stock:{}, StockState:{}",
                stock.as_str(),
                v_json.to_string()
            );
            return Ok(StockStatus::from(stock.as_str()));
        } else {
            info!("请求商品库存失败:{}-{}", status, &prod_id);
            return Ok(StockStatus::Unknown);
        }
    }

    async fn get_prod_price(sku: &str) -> Result<(String, String)> {
        let encoded = form_urlencoded::Serializer::new(String::new())
            .append_pair("type", "1")
            .append_pair("skuIds", format!("J_{}", sku).as_str())
            .finish();
        let resp = reqwest::get(format!("{}?{}", Self::PRICE_URL, encoded)).await?;
        let status = resp.status();
        let r: Value = resp.json::<Value>().await?;
        if r.is_array() && r.as_array().unwrap().len() > 0 {
            let d = &r[0];
            debug!("price:{}-{}", sku, r.to_string());
            let has_data = d.get("p").is_some();
            if has_data {
                return Ok((
                    d["p"]
                        .as_str()
                        .unwrap_or("")
                        .trim_matches(|c| c == '\"')
                        .to_string(),
                    d["op"]
                        .as_str()
                        .unwrap_or("")
                        .trim_matches(|c| c == '\"')
                        .to_string(),
                ));
            }
        }
        warn!("Can't get price info: {}-{}", sku, status);
        Ok(("".to_string(), "".to_string()))
    }

    pub fn get_purchase_info(html: &Html) -> Result<(String, String)> {
        let mut purchase_type = NORMAL.to_string();
        let mut cart_link = String::default();
        'selectors: for selector in &["a#choose-btn-ko", "a#btn-reservation", "a#InitCartUrl"] {
            let purchase_selector = Selector::parse(*selector).unwrap();
            'finder: for element in html.select(&purchase_selector) {
                if let Some(link) = element.value().attr("href") {
                    if let Some(_id) = element.value().attr("id") {
                        // 秒杀的商品
                        if _id.contains("choose-btn-ko") {
                            purchase_type = SECOND_KILL.to_string();
                        } else if _id.contains("btn-reservation") {
                            let txt = element.inner_html();
                            let _txt = txt.trim();
                            if _txt.contains("立即预约") {
                                purchase_type = YUYUE.to_string();
                            } else if _txt.contains("支付定金")
                                || _txt.contains("立即购买")
                            {
                                // https://item.jd.com/100016777664.html  支付定金的预售商品
                                // https://item.jd.com/31335300862.html  立即购买的预售商品
                                purchase_type = PRESALE.to_string();
                            }else if _txt.contains("抢购"){
                                purchase_type = SECOND_KILL.to_string();
                            }
                        }
                    }
                    let link = link.trim();
                    if link.contains("none") {
                        continue 'finder;
                    }
                    if link.starts_with("//") {
                        cart_link = format!("https:{}", link);
                    } else {
                        cart_link = link.to_string();
                    }
                    info!("Found purchase link: {}", &cart_link);
                    break 'selectors;
                }
            }
        }
        Ok((purchase_type, cart_link))
    }

    pub async fn get_goods_info(sku: &str) -> Result<String>{
        let prod_url = format!("https://item.jd.com/{}.html", &sku);
        let resp = default_client("")
            .build()?
            .get(&prod_url)
            .send()
            .await?;
        let txt = resp.text().await?;
        Ok(txt)
    }

    pub async fn get_prod_info(sku: &str, area_id: &str) -> Result<GoodsState> {
        let (goods_info, stock_info, price_info) = join!(
            Self::get_goods_info(sku),
            Self::inner_get_prod_stock(sku, area_id),
            Self::get_prod_price(sku)
        );
        let resp_html = goods_info?;
        let stock_info = stock_info?;
        let price_info = price_info?;
        let mut purchase_type = NORMAL.to_string();
        let mut cart_link = String::new();
        let mut area_id = String::with_capacity(20);
        let mut title = String::with_capacity(60);
        let mut name = String::new();
        {
            let pname = "name:";
            if let Some(idx) = resp_html.find(pname) {
                let txt = resp_html.as_str()[idx + pname.len()..].trim_start();
                let end = "',";
                if let Some(endIdx) = txt.find(end) {
                    let x: &[_] = &['\'', '"', ' '];
                    name = txt[..endIdx].trim_matches(x).to_string();
                }
            }
            let document = Html::parse_document(&resp_html);
            if name.is_empty() {
                let mut name_selector = Selector::parse("div#name h1").unwrap(); // 找到商品名称
                for element in document.select(&name_selector) {
                    name = element.inner_html().trim().to_string();
                    break;
                }
                if name.is_empty() {
                    name_selector = Selector::parse("div.sku-name").unwrap(); // 找到商品名称
                    for element in document.select(&name_selector) {
                        name = element.inner_html().trim().to_string();
                        info!("sku{}, inner_html: {}", sku, &name);
                        if let Some(idx) = name.find(r#">"#) {
                            name = name.split_off(idx + r#">"#.len());
                        }
                        break;
                    }
                }
            }
            // 地区需要根据当前的收货的地址来传
            // 显示商品有货无货需要调用接口
            let info = Self::get_purchase_info(&document)?;
            purchase_type = info.0;
            cart_link = info.1;
        }

        if cart_link.is_empty() {
            if purchase_type.eq(SECOND_KILL) {
                // 秒杀商品在抢购时再获取秒杀链接
            } else if purchase_type.eq(YUYUE) {
                // 预约的商品在加入购物车的时候，才会去获取到预约 开始时间
                // cart_link = format!("https://cart.jd.com/gate.action?pid={}&pcount=1&ptype=1", sku);
            } else {
                cart_link = format!(
                    "https://cart.jd.com/gate.action?pid={}&pcount=1&ptype=1",
                    sku
                );
            }
        }
        let goods_data = GoodsState {
            name: name.trim().to_owned(),
            sku: sku.to_string(),
            cur_price: price_info.0,
            ori_price: price_info.1,
            status: stock_info,
            limit_purchase: 3,
            purchase_num: 1,
            purchase_url: cart_link,
            purchase_type,
            ..Default::default()
        };
        Ok(goods_data)
    }

    pub async fn get_goods_seckill_link(sku: String) -> Result<String> {
        let client = default_client("").build()?;
        Self::get_goods_seckill_link2(sku, client).await
    }

    pub async fn get_goods_seckill_link2(sku: String, client: reqwest::Client) -> Result<String> {
        let seckill_url = {
            let mut rng = thread_rng();
            let jqn: u32 = rng.gen_range(1000000..=9999999);
            format!(
                "https://itemko.jd.com/itemShowBtn?callback=jQuery{}&skuId={}&from=pc&_={}",
                jqn,
                sku.as_str(),
                PKLocal::now().timestamp()
            )
        };
        let resp = client
            .get(seckill_url)
            .header("Host", "itemko.jd.com")
            .header(
                "Referer",
                format!("https://item.jd.com/{}.html", sku.as_str()),
            )
            .send()
            .await?;
        let status = resp.status();
        let mut ret = String::new();
        if status != StatusCode::OK {
            return Ok(ret);
        }
        let v = parse_json(resp.text().await?.as_str());
        let r = v["url"]
            .as_str()
            .unwrap_or("")
            .replace("divide", "marathon")
            .replace("user_routing", "captcha.html");
        if r.starts_with("//") {
            Ok(format!("https:{}", r))
        } else {
            Ok(r)
        }
    }

    pub async fn get_presale_info(sku: String) -> Result<(String, Option<PKDateTime>, String)> {
        let url = format!("https://item.jd.com/{}.html", &sku);
        let mut href = String::new();
        let mut yuyue_dt = None;
        {
            let mut driver = init_driver(false).await?;
            driver.get(url).await?;
            info!("获取预售商品信息：get_presale_info:{}", sku);
            //     btn-reservation
            if let Ok(element) = driver.find_element(By::Id("btn-reservation")).await {
                if let Ok(Some(_href)) = element.get_attribute("href").await {
                    href = _href;
                }
            }
            // 预售商品都有预售时间
            if let Ok(element) = driver.find_element(By::ClassName("J-presale-time")).await {
                //格式 05月24日00:00-05月31日23:30
                let txt = element.text().await.unwrap_or_default();
                info!("可能的预售信息:{}-{}", sku, txt);
                if let Some((start, _)) = txt.split_once('-') {
                    let fmts = vec![
                        format!("%Y年%m月%d日%H:%M"),
                        format!("%Y年%m月%d日%H:%M:%S"),
                    ];
                    let _start;
                    if start.contains("年") {
                        _start = start.to_owned();
                    } else {
                        _start = format!("{}年{}", PKLocal::now().year(), start)
                    }
                    for fmt in &fmts {
                        yuyue_dt = parse_datetime_by_fmt(&_start, &fmt);
                        if yuyue_dt.is_some() {
                            break;
                        }
                    }
                } else {
                    warn!("获取预售时间字符串失败:{}", sku);
                }
            } else {
                warn!("获取预售时间所在元素失败:{}", sku);
            }
            driver.quit().await?;
        }
        Ok((sku, yuyue_dt, href))
    }
}
