use thiserror::Error;

use tide::Error as TideError;
use tide::http::StatusCode;
use log::error;
use std::convert::TryInto;
use std::fmt::{Debug, Display};
pub type Result<T> = std::result::Result<T, MiaoshaError>;

#[derive(Error, Debug)]
pub enum MiaoshaError {
    #[error(transparent)]
    // 发起redis请求时报的错误
    RedisError(#[from] redis::RedisError),

    #[error(transparent)]
    // rbatis 的错误
    RbatisError(#[from] rbatis::Error),

    #[error(transparent)]
    // 程序内部的错误
    OpError(#[from] EOpError),

    #[error(transparent)]
    // 程序内部的错误
    IoError(#[from] std::io::Error),

    // #[error(transparent)]
    // ValidationError(#[from] validator::ValidationError),
    #[error(transparent)]
    ValidationError(#[from] validator::ValidationErrors),

    #[error(transparent)]
    // Json错误
    SerdeJsonError(#[from] serde_json::Error),

    #[error("unknown data store error")]
    Other(#[from] anyhow::Error),
}

impl MiaoshaError {
    pub fn into_tide_error(self, log_error: bool) -> TideError {
        use MiaoshaError::*;
        let ret = {
            match self {
                RedisError(e) => TideError::from_str(StatusCode::InternalServerError, e.to_string()),
                IoError(e) => TideError::from_str(StatusCode::InternalServerError, e.to_string()),
                Other(e) => TideError::from_str(StatusCode::InternalServerError, e.to_string()),
                RbatisError(e) => {
                    let emsg = e.to_string();
                    // 唯一键重复：
                    // Duplicate entry '34-faszxcz-0' for key 'platform_account.ux_userid_account'
                    if emsg.contains("1062") && emsg.contains("ux_userid_account") {
                        TideError::from_str(StatusCode::BadRequest, e.to_string())
                    } else {
                        TideError::from_str(StatusCode::InternalServerError, e.to_string())
                    }
                }

                ValidationError(e) => TideError::from_str(StatusCode::BadRequest, e.to_string()),
                SerdeJsonError(e) => TideError::from_str(StatusCode::BadRequest, e.to_string()),

                OpError(e) => {
                    let emsg = e.to_string();
                    use EOpError::*;
                    match e {
                        Authorization
                        | MissingToken
                        | UnExpectedMacAddr(_, _)
                        | ActivationCodeUsedMoreThanOnce(_)
                        | CookieExpired(_)
                        | ActivationCodeExpired(_) => TideError::from_str(StatusCode::Unauthorized, emsg),

                        CreateActivationCode(_)
                        | ExceedMaxAccountLimits(_)
                        => {
                            TideError::from_str(StatusCode::BadRequest, emsg)
                        }
                    }
                }
            }
        };
        if log_error {
            error!("{}", ret.to_string());
        }
        ret
    }
}


#[derive(Error, Debug)]
pub enum EOpError {
    #[error("Authorize error, need to get activation code!")]
    Authorization,

    #[error("UnExpectedMacAddr error, differnet mac addr: {0} != {1}")]
    UnExpectedMacAddr(String, String),

    #[error("Calling this api need 'token' header!")]
    MissingToken,

    #[error("Can't create account any more because of exceeding max:{0}")]
    ExceedMaxAccountLimits(usize),

    #[error("Cookie expired, need to refresh cookie {0}!")]
    CookieExpired(String),

    #[error("Activation code expired[{0}], need to refresh code !")]
    ActivationCodeExpired(String), // 激活码过期了

    #[error("Activation code[{0}] used more than once, only the latest device could use!")]
    ActivationCodeUsedMoreThanOnce(String), // 激活码超过一个设备在使用了

    #[error("Create activation code failed, level {0}!")]
    CreateActivationCode(u8),
}


pub fn transfer_to_tide_error<S, M>(status: S, msg: M) -> TideError
    where
        S: TryInto<StatusCode>,
        S::Error: Debug,
        M: Display + Debug + Send + Sync + 'static {
    TideError::from_str(status, msg)
}
