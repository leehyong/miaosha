pub mod token;
pub mod check;

use chrono::prelude::*;
use serde::Serializer;
use log::error;
use std::borrow::Borrow;
use tide::http::headers::ToHeaderValues;
use crate::{Local, PKDate, PKDateTime, FixedOffset};


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

pub fn timestamp2datetime(ts: i64) -> PKDateTime {
    PKDateTime::from_utc(
        NaiveDateTime::from_timestamp(ts, 0), FixedOffset::east(0))
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


pub fn parse_datetime(dt: &str) -> Option<PKDateTime> {
    for fmt in [
        DATETIME_FMT,
        DATETIME_FMT1,
        DATETIME_FMT2,
        DATETIME_FMT3,
        DATETIME_FMT4,
        DATETIME_FMT5,
        DATETIME_FMT6,
    ].iter() {
        match NaiveDateTime::parse_from_str(dt, fmt) {
            Ok(dt) => return Some(PKDateTime::from_utc(dt, FixedOffset::east(0))),
            Err(_) => {}
        }
    }
    for fmt in [
        DATE_FMT,
        DATE_FMT1,
    ].iter() {
        match NaiveDate::parse_from_str(dt, fmt) {
            Ok(dt) => {
                let dt = NaiveDateTime::new(dt, NaiveTime::from_hms(0, 0, 0));
                return Some(PKDateTime::from_utc(dt, FixedOffset::east(0)));
            }
            Err(_) => {}
        }
    }
    error!("Unsupported datetime format:{}", dt);
    None
}

pub mod update_create_dt_date_format {
    // copy from https://serde.rs/custom-date-format.html
    // use chrono::{DateTime, Utc, TimeZone};
    use crate::{PKDate, PKDateTime, PKLocal};
    use log::error;
    use chrono::NaiveDateTime;
    use serde::{self, Deserialize, Deserializer, Serializer};
    use crate::utils::parse_datetime;
    use std::error::Error;

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
