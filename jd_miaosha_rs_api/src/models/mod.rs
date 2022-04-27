use serde::{Serialize, Deserialize};
use chrono::Duration;
use redis::AsyncCommands;
use rbatis::crud::CRUD;
use log::info;
use crate::*;
use crate::utils::{update_create_dt_date_format, token, datetime_fmt};
use crate::error::{Result, MiaoshaError, EOpError};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CachedUserInfo {
    pub id: IDType,
    #[serde(with = "VipLevel")]
    pub vip_level: VipLevel,
    #[serde(
    serialize_with = "update_create_dt_date_format::serialize_none",
    deserialize_with = "update_create_dt_date_format::deserialize_pk_dt")]
    pub expire_dt: Option<PKDateTime>,
}

impl CachedUserInfo {
    pub fn is_expired(&self) -> bool {
        if let Some(edt) = self.expire_dt {
            return PKLocal::now() > edt;
        }
        true
    }
}

#[derive(Default, Debug, CRUDTable, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Option<IDType>,
    pub name: String,
    pub mac_addr: String,
    #[serde(with = "VipLevel")]
    pub vip_level: VipLevel,
    pub activate_code: String,
    #[serde(with = "update_create_dt_date_format")]
    pub expire_dt: Option<PKDateTime>,
    #[serde(with = "update_create_dt_date_format")]
    pub create_time: Option<PKDateTime>,
    #[serde(with = "update_create_dt_date_format")]
    pub update_time: Option<PKDateTime>,
}


#[derive(Default, Debug, CRUDTable, Clone, Serialize, Deserialize)]
pub struct PlatformAccount {
    pub id: Option<IDType>,
    pub user_id: IDType,
    pub account: String,
    #[serde(with = "Platform")]
    pub platform: Platform,
    pub pwd: String,
    pub cookie: String,
    #[serde(
    skip_deserializing,
    serialize_with = "update_create_dt_date_format::serialize_none"
    )
    ]
    pub cookie_last_update_dt:Option<PKDateTime>,
    pub eid: String,
    pub fp: String,
    #[serde(with = "update_create_dt_date_format")]
    pub create_time: Option<PKDateTime>,
    #[serde(with = "update_create_dt_date_format")]
    pub update_time: Option<PKDateTime>,
}


#[derive(Default, CRUDTable, Clone, Serialize, Deserialize)]
pub struct LoginHistory {
    pub id: Option<IDType>,
    pub user_id: IDType,
    pub mac_addr: String,
    #[serde(with = "update_create_dt_date_format")]
    pub create_time: Option<PKDateTime>,
}

#[derive(Default, CRUDTable, Clone, Serialize, Deserialize)]
pub struct VipHistory {
    pub id: Option<IDType>,
    pub user_id: IDType,
    #[serde(with = "VipLevel")]
    pub vip_level: VipLevel,
    #[serde(with = "update_create_dt_date_format")]
    pub start_dt: Option<PKDateTime>,
    #[serde(with = "update_create_dt_date_format")]
    pub expire_dt: Option<PKDateTime>,
    #[serde(with = "update_create_dt_date_format")]
    pub create_time: Option<PKDateTime>,
}



#[derive(Default, CRUDTable, Clone, Serialize, Deserialize)]
pub struct ShoppingCart {
    pub id: Option<IDType>,
    pub user_id: IDType,
    #[serde(with = "Platform")]
    pub platform: Platform,
    #[serde(with = "Category")]
    pub category: Category,
    pub sku: String,
    pub name: String,
    pub purchase_num: u32,
    #[serde(
    serialize_with = "update_create_dt_date_format::serialize_none",
    deserialize_with = "update_create_dt_date_format::deserialize_pk_dt")
    ]
    pub yuyue_dt: Option<PKDateTime>,
    #[serde(
    serialize_with = "update_create_dt_date_format::serialize_none",
    deserialize_with = "update_create_dt_date_format::deserialize_pk_dt")
    ]
    pub yuyue_start_dt: Option<PKDateTime>,
    #[serde(
    serialize_with = "update_create_dt_date_format::serialize_none",
    deserialize_with = "update_create_dt_date_format::deserialize_pk_dt")
    ]
    pub yuyue_end_dt: Option<PKDateTime>,
    pub status: String,
    pub ori_price: String,
    pub cur_price: String,
    pub purchase_url: String,
    pub purchase_type: String,
    pub is_stock: u8,
    pub is_delete: u8,
    #[serde(with = "update_create_dt_date_format")]
    pub create_time: Option<PKDateTime>,
    #[serde(with = "update_create_dt_date_format")]
    pub update_time: Option<PKDateTime>,
}

