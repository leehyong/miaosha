use reqwest::Error as RequestError;
use thirtyfour::error::WebDriverError;
use thiserror::Error;
use crate::IDType;

pub type Result<T> = std::result::Result<T, JdMiaoshaError>;

#[derive(Error, Debug)]
pub enum JdMiaoshaError {
    #[error(transparent)]
    // 发起http请求时报的错误
    RequestError(#[from] RequestError),

    #[error(transparent)]
    // chromedriver 的错误
    WebDriver(#[from] WebDriverError),

    #[error(transparent)]
    // 程序内部的错误
    OpError(#[from] OpError),

    #[error(transparent)]
    // 程序内部的错误
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    // 程序内部的错误
    SerdeJsonError(#[from] serde_json::Error),

    #[error("unknown data store error")]
    Other(#[from] anyhow::Error),
}

#[derive(Error, Debug)]
pub enum OpError {
    #[error("Authorize error, need to get activation code!")]
    Authorization,

    #[error("Cookie expired, need to refresh cookie {0}!")]
    CookieExpired(String),

    #[error("Activation code expired[{0}], need to refresh code !")]
    ActivationCodeExpired(String), // 激活码过期了

    #[error("Activation code[{0}] used more than once, only the latest device could use!")]
    ActivationCodeUsedMoreThanOnce(String), // 激活码超过一个设备在使用了

    #[error("The usage of Vip expired, need to refresh vip!")]
    VipExpired(String), // VIP过期了

    #[error("The driver init error!")]
    InitError, // 初始化过期了

    #[error("Account:{0}-Uncheck cart goods error!")]
    UncheckCartGoods(IDType), //

    #[error("Account:{0}-Select cart goods error: {1}!")]
    SelectCartGoods(IDType, String), //

    #[error("{0}-{1}, add cart goods error!")]
    AddCartGoods(IDType, String), //

    #[error("Create order info error!")]
    CreateOrderInfo,

    #[error("Submit order error!")]
    SubmitOrder, // 提交订单失败

    #[error("Remove goods from cart error!")]
    RemoveCartGoods,

    #[error("Get shoppincart info error!")]
    GetCartInfo,
}
