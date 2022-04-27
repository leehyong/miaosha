use crate::IDType;
use jsonwebtoken::*;
use serde::{Serialize, Deserialize};
use log::{info, error};

#[derive(Serialize, Deserialize)]
struct Claims {
    pwd:String
}

static PWD_SECRET:&'static str = "8AkUYMx3CbWt5VyZnLVWTEofKYolQ13Jkxd";


pub(crate) fn create_secret_pwd(pwd:String) -> String {
    let my_claims = Claims {
        pwd,
    };

    match encode(&Header::default(), &my_claims,
                 &EncodingKey::from_secret(PWD_SECRET.as_bytes())) {
        Ok(t) =>t,
        Err(e) => {
            error!("{:?}", e);
            "".to_string()
        }
    }
}

pub(crate) fn decode_secret_pwd(data: &str) -> String {
    let mut v = Validation::default();
    v.validate_exp = false;
    match decode::<Claims>(data,
                           &DecodingKey::from_secret(PWD_SECRET.as_bytes()),
                           &v) {
        Ok(token) => token.claims.pwd,
        Err(e) => {
            error!("decode_code error: {:?}", e);
            "".to_string()
        }
    }
}