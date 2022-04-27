mod goods;
mod area;
mod user;
mod address;
mod shopping_cart;
mod order;
pub use goods::{GoodsState, StockStatus};
pub use area::{Area, PROVINCES, PROVINCE_NAMES, DEFAULT_ADDR, DEFAULT_ADDR_NAMES};
pub use user::{UserState, UserInfo, UserInfoStatus, AccountsPageState};
pub use address::{AddressInfo};
pub use shopping_cart::{CartProdState, CartProdStatus, ShoppingCartPageState, PInfo, YuyueInfo};
pub use order::OrderInfo;

pub use crate::types::*;