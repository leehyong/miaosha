use serde::{Serialize, Deserialize};
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use chrono::Duration;
use log::{error, info, debug};
use redis::AsyncCommands;
use crate::models::CachedUserInfo;
use crate::error::{Result, MiaoshaError, EOpError};
use crate::{PKDateTime, PKLocal, IDType, get_redis_conn};
use crate::access::{Role, Permission};

const PRIVATE_KEY: &'static str = "ruwoeiruwoccxczd5646";


#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    exp: usize,
    user_id: u32,
    level: u8,
    now: i64,
}

impl Claims{
    pub fn permissions(&self)->Vec<String>{
        // vec![
        //     Permission::Delete,
        //     Permission::Read,
        //     Permission::Write,
        //     Permission::Update,
        // ]
        vec![
            "Delete".to_string(),
            "Read".to_string(),
            "Write".to_string(),
            "Update".to_string(),
        ]
    }
    pub fn role(&self)->&'static str{
        // vec![
        //     Permission::Delete,
        //     Permission::Read,
        //     Permission::Write,
        //     Permission::Update,
        // ]
        "Super"
    }
}


pub(crate) fn create_token(user_id: IDType, level:u8) -> Option<String> {
    let my_claims = Claims {
            user_id,
            level,
            exp: 10000000000000000,
            now:PKLocal::now().timestamp_millis()
        };
    match encode(&Header::default(), &my_claims,
                 &EncodingKey::from_secret(PRIVATE_KEY.as_bytes())) {
        Ok(t) =>{
            Some(format!("{:X}", md5::compute(t.as_bytes())))
        },
        Err(e) => {
            error!("{:?}", e);
            None
        }
    }
}
//
// pub(crate) fn decode_token(code: &str) -> Option<Claims> {
//     match decode::<Claims>(code,
//                            &DecodingKey::from_secret(PRIVATE_KEY.as_bytes()),
//                            &Validation::default()) {
//         Ok(token) => {
//             Some(token.claims)
//         }
//         Err(e) => {
//             error!("decode_code error: {:?}", e);
//             None
//         }
//     }
// }

pub fn activate_code_key(code:&str) -> String{
    format!("activate_code:{}", code)
}


pub async fn get_cached_user(code:&str, is_activate:bool) -> Result<CachedUserInfo>{
    let mut con = get_redis_conn().await?;
    let key = activate_code_key(code);
    debug!("{}", key.as_str());
    let s:redis::RedisResult<String> = con.get(key.as_str()).await;
    match s{
        Ok(s) => {
            debug!("{}", s.as_str());
            let u: CachedUserInfo = serde_json::from_str(s.as_str())?;
            if u.is_expired(){
                debug!("{:?}, condition:{}-{}", u, is_activate, u.expire_dt.is_none());
                if is_activate && u.expire_dt.is_none(){
                    // 激活码未激活时，是没有填过期时间的
                    return Ok(u);
                }
                debug!("is_expired:{}", u.id);
                return Err(MiaoshaError::OpError(EOpError::ActivationCodeExpired(code.to_owned())));
            }
            Ok(u)
        }
        Err(e) =>{
            error!("{},{}", key, e.to_string());
            return Err(MiaoshaError::OpError(EOpError::ActivationCodeExpired(code.to_owned())));
        }
    }
}


