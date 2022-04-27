use crate::utils::*;
use crate::*;
use iced::{button, text_input};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::collections::LinkedList;
use std::sync::Arc;
use mac_address::get_mac_address;


#[derive(Debug, Clone, Deserialize)]
pub struct AccountsPageState{
    pub page_no:i64,
    pub page_size:i64,
    pub total:i64,
    pub pages:i64,
    pub records:Vec<UserState>,
    #[serde(skip)]
    pub timestamp:i64
}

impl PartialEq for AccountsPageState{
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}

impl Default for AccountsPageState{
    fn default() -> Self {
        Self{
            timestamp:Local::now().timestamp(),
            page_no:1,
            pages:0,
            page_size:10,
            total:0,
            records:vec![]
        }
    }
}

impl PartialOrd for AccountsPageState{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.timestamp > other.timestamp{
            Some(Ordering::Greater)
        }else if self.timestamp == other.timestamp{
            Some(Ordering::Equal)
        }else{
            Some(Ordering::Less)
        }
    }
}


#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct UserState {
    pub id: IDType,
    pub account: String,
    pub pwd: String,
    #[serde(serialize_with = "cookie_serialize", deserialize_with="cookie_deserialize")]
    pub cookie: Arc<String>, //  todo cookie 使用读写锁要做到整个app共享
    pub eid: String,
    pub fp: String,
    #[serde(skip)]
    pub is_view_passwd: bool,
    #[serde(skip)]
    pub is_selected: bool,
    // 上次 cookie 更新时间
    #[serde(
    serialize_with="update_create_dt_date_format::serialize_none",
    deserialize_with="update_create_dt_date_format::deserialize_pk_dt"
    )]
    pub cookie_last_update_dt: Option<PKDateTime>,
    #[serde(skip)]
    pub is_select_button_state: bool,
    #[serde(skip)]
    pub check_button_state: button::State,
    #[serde(skip)]
    pub login_button_state: button::State,
    #[serde(skip)]
    pub view_pwd_button_state: button::State,
    #[serde(skip)]
    pub update_button_state: button::State,
    #[serde(skip)]
    pub delete_button_state: button::State,
    #[serde(skip)]
    pub account_input_state: text_input::State,
    #[serde(skip)]
    pub pwd_input_state: text_input::State,
    #[serde(skip)]
    pub status: UserInfoStatus,
}

fn cookie_serialize<S>(t:&Arc<String>, serializer:S) -> Result<S::Ok, S::Error>
    where S: Serializer{
    serializer.serialize_str( t.as_str())
}

fn cookie_deserialize<'de, D>(de:D) -> Result<Arc<String>, D::Error>
    where D: Deserializer<'de>
    {
        let s = String::deserialize(de)?;
        Ok(Arc::new(s))
}




#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum UserInfoStatus {
    NewIng,
    Editing,
    CookieChecking,
    Done,
}

impl Default for UserInfoStatus {
    fn default() -> Self {
        UserInfoStatus::Done
    }
}

impl PartialEq for UserState {
    fn eq(&self, other: &Self) -> bool {
        self.account == other.account
    }
}

impl PartialOrd for UserState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.account < other.account {
            Some(Ordering::Less)
        } else if self.account == other.account {
            Some(Ordering::Equal)
        } else {
            Some(Ordering::Greater)
        }
    }
}



#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserInfo {
    // 通过激活码、mac 来让本程序做到单点登录
    // 激活码
    pub activate_code: String,
    #[serde(rename="mac_addr")]
    pub mac: String, // 激活码最近使用的mac地址
    #[serde(rename="id")]
    pub user_id:IDType,
    #[serde(rename="vip_level")]
    pub level:u8,
    // 过期时间
    #[serde(
    serialize_with = "update_create_dt_date_format::serialize_none",
    deserialize_with = "update_create_dt_date_format::deserialize_pk_dt")
    ]
    #[serde(rename="expire_dt")]
    pub expire_date: Option<PKDateTime>,
    // jd账号列表
    #[serde(skip)]
    pub users: AccountsPageState,
}


impl PartialEq for UserInfo {
    fn eq(&self, other: &Self) -> bool {
        self.user_id == other.user_id
    }
}

impl PartialOrd for UserInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.user_id < other.user_id {
            Some(Ordering::Less)
        } else if self.user_id == other.user_id {
            Some(Ordering::Equal)
        } else {
            Some(Ordering::Greater)
        }
    }
}

impl UserInfo {

    pub const FREE_TRY:u8 = 0;
    pub const VIP1:u8 = 1;
    pub const VIP2:u8 = 2;
    pub const VIP3:u8 = 3;
    pub const VIP4:u8 = 4;
    pub const VIP5:u8 = 5;
    pub const VIP6:u8 = 6;
    pub const VIP7:u8 = 7;
    pub const VIP8:u8 = 8;
    pub const VIP9:u8 = 9;


    pub fn is_expired(&self) -> bool {
        if let Some(dt) = self.expire_date {
            // 过期时间没有的话， 就默认是过期了
            if dt > PKLocal::now() {
                // 没有过期
                return false;
            }
        }
        // 过期了
        return true;
    }
    const DEFAULT_MAC_ADDRESS: &'static str = "A4:5E:60:BF:57:E7";

    pub fn get_user_mac_address() -> String{
        match get_mac_address(){
            Ok(m) => {
                match m{
                    Some(_mac) => format!("{}", _mac),
                    None => Self::DEFAULT_MAC_ADDRESS.to_string()
                }
            }
            Err(_) => {
                Self::DEFAULT_MAC_ADDRESS.to_string()
            }
        }
    }

    pub fn version_level(level:u8)-> &'static str{
        match level {
            Self::VIP1 => "vip1",
            Self::VIP2 => "vip2",
            Self::VIP3 => "vip3",
            Self::VIP4 => "vip4",
            Self::VIP5 => "vip5",
            Self::VIP6 => "vip6",
            Self::VIP7 => "vip7",
            Self::VIP8 => "vip8",
            Self::VIP9 => "vip9",
            _ => "试用"
        }
    }

    pub fn expire_date_seconds(level:u8)-> i64{
        const DAY_SECONDS:i64 = 24*3600;
        match level {
            Self::VIP1 => 15 * DAY_SECONDS,
            Self::VIP2 => 30 * DAY_SECONDS,
            Self::VIP3 => 60 * DAY_SECONDS,
            Self::VIP4 => 90 * DAY_SECONDS,
            Self::VIP5 => 180 * DAY_SECONDS,
            Self::VIP6 => 300 * DAY_SECONDS,
            Self::VIP7 => 600 * DAY_SECONDS,
            Self::VIP8 => 1000 * DAY_SECONDS,
            Self::VIP9 => 2000 * DAY_SECONDS,
            _ =>  3 * DAY_SECONDS
        }
    }
}
