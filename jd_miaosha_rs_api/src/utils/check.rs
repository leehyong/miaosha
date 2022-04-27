use crate::error::*;
use crate::*;

use tide::{StatusCode};

fn missing_token_header() -> MiaoshaError {
    MiaoshaError::OpError(EOpError::MissingToken)
}

pub fn check_token_header(req: &MRequest) -> std::result::Result<String, tide::Error>
     {
    if let Some(code) = req.header("token") {
        if let Some(_code) = code.get(0){
            return Ok(_code.to_string());
        }
    }
    Err(missing_token_header().into_tide_error(false))
}

pub fn check_id(req: &MRequest) -> std::result::Result<IDType, tide::Error>
     {
    let id: IDType = req.param("id")?.parse()?;
    if id < 1 {
        return Err(transfer_to_tide_error(
            StatusCode::BadRequest, format!("参数id:{}不能小于 1", id)));
    }
    Ok(id)
}

