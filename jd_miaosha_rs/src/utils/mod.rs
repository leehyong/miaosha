use std::borrow::Borrow;
use std::cmp::{max, min};
use std::option::Option::Some;

use chrono::prelude::*;
use http::HeaderValue;
use log::error;
use serde::Serializer;

pub use proxy_ip::proxy_client_builder;

use crate::*;

pub mod icon;
pub mod img;
pub mod helper;
mod proxy_ip;

const DATE_FMT: &'static str = "%Y-%m-%d";
const DATE_FMT1: &'static str = "%Y/%m/%d";
const DATETIME_FMT: &'static str = "%Y-%m-%d %H:%M:%S";
const DATETIME_FMT1: &'static str = "%Y-%m-%d %H:%M";
const DATETIME_FMT2: &'static str = "%Y/%m/%d %H:%M:%S";
const DATETIME_FMT3: &'static str = "%Y/%m/%d %H:%M";
const DATETIME_FMT4: &'static str = "%Y/%m/%dT%H:%M";
const DATETIME_FMT5: &'static str = "%Y/%m/%dT%H:%M:%S";
const DATETIME_FMT6: &'static str = "%Y-%m-%dT%H:%M:%S";

pub fn date_fmt<T: Borrow<PKDate>>(t: T) -> String {
    t.borrow().format(DATE_FMT).to_string()
}

pub fn datetime_fmt<T: Borrow<PKDateTime>>(t: T) -> String {
    t.borrow().format(DATETIME_FMT).to_string()
}

pub fn datetime_fmt_option(t: &Option<PKDateTime>) -> String {
    if let Some(t) = t{
        datetime_fmt(t)
    }else{
        "".to_string()
    }
}

pub fn greater_than_now(dt:&PKDateTime) -> bool{
    // 提前判定 ahead_purchase_millis() 豪秒购买，可能会提高抢购成功率
    dt.timestamp_millis() - ahead_purchase_millis() > now().timestamp_millis()
}

pub fn now() -> PKDateTime{
    PKDateTime::from_utc(PKLocal::now().naive_local(), FixedOffset::east(0))
}

pub fn workers() ->usize{
    let workers = env::var("workers")
        .unwrap_or("15".to_string()).parse::<usize>()
        .unwrap_or(15);
    // 5 <= workers <= 1000
    min(100, max(5, workers))
    // 对购物车都是加锁的， 并发抢没得意义，暂时去掉，等待找到更好的方式
    // 1
}

pub fn ahead_purchase_millis() ->i64{
    let ahead_purchase_millis = env::var("ahead")
        .unwrap_or("1200".to_string()).parse::<i64>()
        .unwrap_or(1200);
    min(10000, max(ahead_purchase_millis, 500))
}

pub fn timestamp2datetime(ts: i64) -> PKDateTime {
    PKDateTime::from_utc(
        NaiveDateTime::from_timestamp(ts, 0), FixedOffset::east(0))
}

pub async fn sleep(millis:u64) {
    tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
}

pub fn sleep_until(instant:tokio::time::Instant) -> tokio::time::Sleep{
    tokio::time::sleep_until(instant)
}

pub fn timestamp2datetime_str(ts: Option<i64>) -> String {
    datetime_fmt(timestamp2datetime(ts.unwrap_or(0)))
}

pub fn serialize_timestamp2datetime_str<S>(ts: &Option<i64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
{
    if let Some(t) = ts {
        return serializer.serialize_str(timestamp2datetime_str(Some(*t)).as_str());
    }
    serializer.serialize_none()
}

pub fn parse_json(txt:&str) -> serde_json::Value{
    if txt.len() < 2 {
        return serde_json::json!({});
    }
    let left = txt.find('{').unwrap_or(0);
    let right = txt.rfind ('}').unwrap_or(txt.len() - 1);
    serde_json::from_str(&txt[left..=right]).unwrap_or(serde_json::json!({}))
}

pub fn get_useragent() -> &'static str{
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 11_1_0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/89.0.4389.114 Safari/537.36"
}

pub fn default_client(cookie:&str) -> reqwest::ClientBuilder{
    let mut dh = reqwest::header::HeaderMap::new();
    dh.insert("user-agent", HeaderValue::from_static(get_useragent()));
    dh.insert("Connection", HeaderValue::from_static("keep-alive"));
    dh.insert("cookie", HeaderValue::from_str(cookie).unwrap());
    reqwest::Client::builder()
        .cookie_store(true)
        .default_headers(dh)
}

pub fn parse_datetime(dt: &str) -> Option<PKDateTime> {
    let dt = dt.replace("：", ":");
    for fmt in [
        DATETIME_FMT,
        DATETIME_FMT1,
        DATETIME_FMT2,
        DATETIME_FMT3,
        DATETIME_FMT4,
        DATETIME_FMT5,
        DATETIME_FMT6,
    ].iter() {
        match NaiveDateTime::parse_from_str(&dt, fmt) {
            Ok(dt) => return Some(PKDateTime::from_utc(dt, FixedOffset::east(0))),
            Err(_) => {}
        }
    }
    for fmt in [
        DATE_FMT,
        DATE_FMT1,
    ].iter() {
        match NaiveDate::parse_from_str(&dt, fmt) {
            Ok(dt) => {
                let dt = NaiveDateTime::new(dt, NaiveTime::from_hms(0, 0, 0));
                return Some(PKDateTime::from_utc(dt, FixedOffset::east(0)));
            }
            Err(_) => {}
        }
    }
    error!("Unsupported datetime:{}", dt);
    None
}

pub fn parse_datetime_by_fmt(dt: &str, fmt:&str) -> Option<PKDateTime> {
    match NaiveDateTime::parse_from_str(dt, fmt) {
        Ok(dt) => return Some(PKDateTime::from_utc(dt, FixedOffset::east(0))),
        Err(_) => {
            match NaiveDate::parse_from_str(dt, fmt) {
                Ok(dt) => {
                    let dt = NaiveDateTime::new(dt, NaiveTime::from_hms(0, 0, 0));
                    return Some(PKDateTime::from_utc(dt, FixedOffset::east(0)));
                }
                Err(_) => {
                    error!("Unsupported datetime:{}", dt);
                    None
                }
            }
        }
    }
}

pub mod update_create_dt_date_format {
    use serde::{self, Deserialize, Deserializer, Serializer};

    // copy from https://serde.rs/custom-date-format.html
    // use chrono::{DateTime, Utc, TimeZone};
    use crate::{PKDateTime, PKLocal};
    use crate::utils::parse_datetime;

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &Option<PKDateTime>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        if let Some(dt) = date {
            let s = format!("{}", dt.format(super::DATETIME_FMT));
            serializer.serialize_str(&s)
        } else {
            serializer.serialize_str(PKLocal::now().format(super::DATETIME_FMT).to_string().as_str())
        }
    }

    pub fn serialize_none<S>(date: &Option<PKDateTime>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        if let Some(dt) = date {
            let s = format!("{}", dt.format(super::DATETIME_FMT));
            serializer.serialize_str(&s)
        } else {
            serializer.serialize_none()
        }
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<PKDateTime>, D::Error>
        where
            D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(parse_datetime(s.as_str()))
    }

    pub fn deserialize_pk_dt<'de, D>(deserializer: D) -> Result<Option<PKDateTime>, D::Error>
        where
            D: Deserializer<'de>,
    {
        match String::deserialize(deserializer){
            Ok(s) =>{
                if s.len() > 0{
                    Ok(parse_datetime(s.as_str()))
                }else{
                    Ok(None)
                }
            }
            Err(e) =>{
                let msg = format!("{:?}", e);
                if msg.contains("null"){
                    Ok(None)
                }else{
                    Err(e)
                }
            }
        }
    }
}
