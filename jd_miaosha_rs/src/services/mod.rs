pub mod area;
pub mod delivery_address;
pub mod driver;
pub mod goods;
pub mod order;
pub mod reqwest_async;
pub mod shopping_cart;
pub mod user;

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use crate::IDType;

lazy_static!(
    static ref SESSION_COOKIES:Arc<RwLock<HashMap<IDType, Arc<String>>>> = Arc::new(RwLock::new(HashMap::new()));
);
