mod user;
mod shopping_cart;

use validator::Validate;
use serde::Deserialize;
pub use user::*;
pub use shopping_cart::*;

#[derive(Clone, Validate, Deserialize, Debug)]
pub struct KeyWordPageReq {
    #[validate(length(min = 2))]
    pub key_word: Option<String>,
    #[validate(range(min = 1))]
    pub page_no: Option<u64>,
    #[validate(range(min = 5, max = 30))]
    pub page_size: Option<u64>,
}
