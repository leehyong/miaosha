use crate::utils::{datetime_fmt, update_create_dt_date_format};
use crate::*;
use iced::{button, text_input};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::collections::LinkedList;
use std::sync::Arc;
use super::{ProdPlatform, Category};


#[derive(Debug,Copy, Clone, PartialOrd, PartialEq, Deserialize, Serialize)]
pub enum CartProdStatus{
    Done,
    Editing,
}

impl Default for CartProdStatus{
    fn default() -> Self {
        Self::Done
    }
}
//

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CartProdState {
    #[serde(skip_serializing)]
    pub id:IDType,
    pub user_id: IDType,
    pub sku: String,
    pub name: String,
    pub purchase_num: u32,
    pub purchase_url: String,
    pub purchase_type: PurchaseType,
    #[serde(with = "ProdPlatform")]
    pub platform: ProdPlatform,
    #[serde(with = "Category")]
    pub category: Category,
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
    #[serde(
    serialize_with = "update_create_dt_date_format::serialize_none",
    deserialize_with = "update_create_dt_date_format::deserialize_pk_dt")
    ]
    pub yuyue_dt: Option<PKDateTime>,
    #[serde(rename="status")]
    pub purchase_status: String,
    pub is_stock: u8,  // 是否有货
    pub ori_price: String,
    pub cur_price: String,
    #[serde(skip_serializing,with = "update_create_dt_date_format")]
    pub create_time: Option<PKDateTime>,
    #[serde(skip_serializing,with = "update_create_dt_date_format")]
    pub update_time: Option<PKDateTime>,

    // 预约购买时间
    #[serde(skip)]
    pub yuyue_txt:String,
    #[serde(skip)]
    pub cur_check:usize,
    #[serde(skip)]
    pub is_selected: bool,
    #[serde(skip)]
    // 预约购买按钮
    pub timeout_button_state: button::State,
    #[serde(skip)]
    pub check_button_state: button::State,
    #[serde(skip)]
    // 立即购买按钮
    pub immediately_button_state: button::State,
    #[serde(skip)]
    pub add_num_button_state: button::State,
    #[serde(skip)]
    pub sub_num_button_state: button::State,
    // 预约购买时间
    #[serde(skip)]
    pub timeout_dt_input_state: text_input::State,
    #[serde(skip)]
    // 删除
    pub delete_button_state: button::State,
    #[serde(skip)]
    pub status: CartProdStatus,
}


impl PartialEq for CartProdState {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for CartProdState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.id < other.id {
            Some(Ordering::Less)
        } else if self.id == other.id {
            Some(Ordering::Equal)
        } else {
            Some(Ordering::Greater)
        }
    }
}



#[derive(Debug, Clone, Deserialize)]
pub struct ShoppingCartPageState{
    pub page_no:i64,
    pub page_size:i64,
    pub pages:i64,
    pub total:i64,
    pub records:Vec<CartProdState>,
    #[serde(skip)]
    pub timestamp:i64
}

impl PartialEq for ShoppingCartPageState{
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}

impl Default for ShoppingCartPageState{
    fn default() -> Self {
        Self{
            timestamp:PKLocal::now().timestamp(),
            page_no:1,
            pages:0,
            page_size:20,
            total:0,
            records:vec![]
        }
    }
}

impl PartialOrd for ShoppingCartPageState{
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
#[allow(non_snake_case)]
pub struct PInfo{
    pub skuId:String,
    pub skuType:String,
    pub brandId:String,
    pub skuName:String,
    pub pName:String,
    pub productArea:String,
    pub model:String,
    pub venderID:String,
    pub category:Vec<String>,

}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct YuyueInfo{
    pub cart_goods_id:IDType,
    pub yuyue_start_dt: Option<PKDateTime>,
    pub yuyue_end_dt: Option<PKDateTime>,
    pub qiang_start_dt: Option<PKDateTime>,
    pub yuyue_url:String,

}
