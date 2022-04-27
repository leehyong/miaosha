use serde::Deserialize;
use validator::{Validate, ValidationError};
use crate::{Platform, IDType};

#[derive(Copy, Clone, Validate, Deserialize)]
pub struct CreateActivateCodeReq {
    #[validate(range(min = 0, max = 9))]
    pub level: u8
}


fn valid_activate(_: &ActivateReq) -> Result<(), ValidationError> {
    Ok(())
}

#[derive(Clone, Validate, Deserialize)]
#[validate(schema(function = "valid_activate", skip_on_field_errors = false))]
pub struct ActivateReq {
    #[validate(length(equal = 32))]
    pub activate_code: String,
    #[validate(length(min = 10, max = 40))]
    pub mac: String,
}


#[derive(Clone, Validate, Deserialize)]
pub struct RefreshReq {
    #[validate(length(min = 1))]
    pub codes: Vec<String>,
}


#[derive(Clone, Validate, Deserialize)]
pub struct AddAccountReq {
    #[validate(length(min = 2))]
    pub account: String,
    #[validate(length(min = 6))]
    pub pwd: String,
}


#[derive(Clone, Validate, Deserialize)]
pub struct UpdateAccountReq {
    #[validate(length(min = 2))]
    pub account: Option<String>,
    #[validate(length(min = 6))]
    pub pwd: Option<String>,
    #[validate(length(min = 1))]
    pub eid: Option<String>,
    #[validate(length(min = 1))]
    pub fp: Option<String>,
    #[validate(length(min = 10))]
    pub cookie: Option<String>,
}


#[derive(Clone, Validate, Deserialize, Debug)]
pub struct BatchDeleteAccountReq {
    #[validate(length(min = 1))]
    pub ids: Option<Vec<IDType>>,
}

#[derive(Clone, Validate, Deserialize, Debug)]
pub struct HeartbeatReq {
    #[validate(length(min = 10, max = 40))]
    pub mac: String,
}
