use crate::{PurchaseType, PKDateTime};
use iced::button;
use std::cmp::Ordering;
use std::fmt::Formatter;

#[derive(Debug, Clone, Default)]
pub struct GoodsState {
    pub name: String,
    pub sku: String,
    pub ori_price: String,
    pub cur_price: String,
    pub status: StockStatus,
    pub limit_purchase: u8,
    pub purchase_num: u8,
    pub purchase_url: String,
    pub yuyue_dt: Option<PKDateTime>,
    pub purchase_type: PurchaseType,
    pub add_num_button_state: button::State,
    pub sub_num_button_state: button::State,
    pub immediately_buy_button_state: button::State,
    pub add_to_cart_button_state: button::State,
    pub timeout_purchase_button_state: button::State,
}

#[derive(Copy, Clone, Debug)]
pub enum StockStatus {
    Distribution,
    OnSale,
    OutOfStock,
    PreSell,
    Unknown,
}

impl Default for StockStatus {
    fn default() -> Self {
        Self::OnSale
    }
}

impl StockStatus {
    pub fn is_stock(&self) -> bool {
        // 判定有货无货
        match self {
            StockStatus::Distribution | StockStatus::OnSale | StockStatus::PreSell => true,
            _ => false,
        }
    }
    pub fn is_stock_u8(&self) -> u8 {
        if self.is_stock(){
            1
        }else{
            0
        }
    }
}

impl std::fmt::Display for StockStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StockStatus::Distribution => write!(f, "可配货"),
            StockStatus::OnSale => write!(f, "有货"),
            StockStatus::OutOfStock => write!(f, "无货"),
            StockStatus::PreSell => write!(f, "采购中"),
            StockStatus::Unknown => write!(f, "未知"),
        }
    }
}
impl From<&str> for StockStatus {
    fn from(status: &str) -> Self {
        if status == "40" || status == "39" {
            Self::Distribution
        } else if status == "33" {
            Self::OnSale
        } else if status == "34" || status == "0" {
            Self::OutOfStock
        } else if status == "36" {
            Self::PreSell
        } else {
            Self::Unknown
        }
    }
}

impl PartialEq for StockStatus {
    fn eq(&self, other: &Self) -> bool {
        use StockStatus::*;
        match (self, other) {
            (Distribution, Distribution)
            | (OnSale, OnSale)
            | (OutOfStock, OutOfStock)
            | (PreSell, PreSell) => true,
            _ => false,
        }
    }
}

impl PartialOrd for StockStatus{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // 只是占位方法， 代码里不会用来判断大小， 姑且认为都是相等的。
        Some(Ordering::Equal)
    }
}

impl PartialEq for GoodsState {
    fn eq(&self, other: &Self) -> bool {
        self.sku == other.sku
    }
}

impl PartialOrd for GoodsState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.sku < other.sku {
            Some(Ordering::Less)
        } else if self.sku == other.sku {
            Some(Ordering::Equal)
        } else {
            Some(Ordering::Greater)
        }
    }
}
