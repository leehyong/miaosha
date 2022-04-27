use iced::{button, text_input};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::sync::Arc;


#[derive(Debug, Clone, Default)]
pub struct AddressInfo {
    pub account: String,
    pub cookie: Arc<String>,
    pub addr_id: String,
    pub receiver: String,
    // 地区
    pub area_zone: String,
    // 地址
    pub address: String,
    // 手机
    pub mobile_phone: String,
    // 固定电话
    pub fixed_line_phone: String,
    pub email: String,
    // 最新的收货地址
    pub is_latest_receive_addr: bool,
    // 设置收货地址按钮
    pub set_addr_button_state: button::State,
}


impl PartialEq for AddressInfo {
    fn eq(&self, other: &Self) -> bool {
        self.addr_id == other.addr_id
    }
}

impl PartialOrd for AddressInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.addr_id < other.addr_id {
            Some(Ordering::Less)
        } else if self.addr_id == other.addr_id {
            Some(Ordering::Equal)
        } else {
            Some(Ordering::Greater)
        }
    }
}
