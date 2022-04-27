use iced::{button, text_input};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::collections::LinkedList;
use std::sync::Arc;
use super::ProdPlatform;


#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct OrderInfo {
    pub account: String,
    pub order_no: String,
    pub name: String,
    pub status: String,
    pub purchase_num: String,
    pub total_price: String,
    pub receiver: String,
    pub create_dt: String,
    #[serde(with="ProdPlatform")]
    pub platform: ProdPlatform,
    #[serde(skip)]
    pub cookie: Arc<String>, // todo cookie 使用读写锁要做到整个app共享
    #[serde(skip)]
    pub express_button_state: button::State,
}

impl PartialEq for OrderInfo {
    fn eq(&self, other: &Self) -> bool {
        self.order_no == other.order_no
    }
}

impl PartialOrd for OrderInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.order_no < other.order_no {
            Some(Ordering::Less)
        } else if self.order_no == other.order_no {
            Some(Ordering::Equal)
        } else {
            Some(Ordering::Greater)
        }
    }
}

