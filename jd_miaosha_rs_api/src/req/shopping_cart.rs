use serde::Deserialize;
use validator::{Validate, ValidationError};
use crate::{Platform, IDType, PKDateTime};
use crate::utils::update_create_dt_date_format;

#[derive(Clone, Validate, Deserialize)]
pub struct AddGoodsCartReq {
    #[validate(length(min = 1))]
    pub sku: String,
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(range(min = 1))]
    pub purchase_num: u32,

    #[validate(range(min = 0, max=1))]
    pub is_stock: u8,
    #[serde(
    serialize_with = "update_create_dt_date_format::serialize_none",
    deserialize_with = "update_create_dt_date_format::deserialize_pk_dt")
    ]
    pub yuyue_dt: Option<PKDateTime>,
    #[validate(length(min = 1))]
    pub ori_price: String,
    #[validate(length(min = 1))]
    pub cur_price: String,
    pub purchase_url: String,
    pub purchase_type: String,
}

#[derive(Clone, Validate, Deserialize)]
pub struct UpdateGoodsCartUrlReq {
    #[validate(length(min = 1))]
    pub sku: String,
    #[validate(length(min = 1))]
    pub purchase_url: String,
}

fn validate_req(req: &UpdateGoodsCartReq) -> Result<(), ValidationError> {
    if req.op == 4{
        if req.yuyue_dt.is_none(){
            return Err(ValidationError::new("Please type the yuyue_dt!"))
        }
    }
    Ok(())
}

#[derive(Copy, Clone, Validate, Deserialize)]
#[validate(schema(function = "validate_req", skip_on_field_errors = false))]
pub struct UpdateGoodsCartReq {
    #[validate(range(min = 1))]
    pub op:u8, // 操作码: 1,加；2减；3，立即购买；4，预约购买 5, 成功; 6, 失败; 7, 预约中
    #[serde(
    serialize_with = "update_create_dt_date_format::serialize_none",
    deserialize_with = "update_create_dt_date_format::deserialize_pk_dt")
    ]
    pub yuyue_dt: Option<PKDateTime>,
}


#[derive(Copy, Clone, Validate, Deserialize)]
pub struct UpdateYuyueGoodsCartReq {
    #[validate(range(min = 1, max=6))]
    pub op:u8, // 操作码: 1,加；2减；3，立即购买；4，预约购买 5, 成功; 6, 失败
    #[serde(
    serialize_with = "update_create_dt_date_format::serialize_none",
    deserialize_with = "update_create_dt_date_format::deserialize_pk_dt")
    ]
    pub yuyue_dt: Option<PKDateTime>,
    #[serde(
    serialize_with = "update_create_dt_date_format::serialize_none",
    deserialize_with = "update_create_dt_date_format::deserialize_pk_dt")
    ]
    pub yuyue_start_dt: Option<PKDateTime>,
    #[serde(
    serialize_with = "update_create_dt_date_format::serialize_none",
    deserialize_with = "update_create_dt_date_format::deserialize_pk_dt")
    ]
    pub yuyue_end_dt: Option<PKDateTime>,
}


#[derive(Clone, Validate, Deserialize, Debug)]
pub struct BatchDeleteGoodsCartReq {
    #[validate(length(min = 1))]
    pub ids: Option<Vec<IDType>>,
}
