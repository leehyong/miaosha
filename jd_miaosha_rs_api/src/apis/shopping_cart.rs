use tide::Server;
use tide::StatusCode;
use log::error;
use validator::Validate;
use crate::models::User;
use crate::req::*;
use crate::utils::{token, check};
use crate::services::shopping_cart;
use crate::error::*;
use crate::*;


pub fn api(app: &mut Server<AppState>)
{
    let mut service = tide::with_state(app.state().clone());
    service
        .at("/:id")
        .put(update_goods_cart)
        .delete(delete_goods_cart);
    service.at("/yuyue/:id")
        .put(update_yuyue_goods_cart);
    app.at("/api/shopping_cart")
        .get(list_goods_cart)
        .post(add_goods_cart)
        .put(update_goods_cart_purchase_url)
        .delete(batch_delete_goods_cart)
        .nest(service);
}


async fn add_goods_cart(mut req: MRequest) -> tide::Result
     {
    let code = check::check_token_header(&req)?;
    let query: AddGoodsCartReq = req.body_json().await?;
    shopping_cart::add_goods_cart(code, query)
        .await.map_err(|e| e.into_tide_error(false))?;
    Ok("".into())
}

async fn update_goods_cart(mut req: MRequest) -> tide::Result
     {
    let id = check::check_id(&req)?;
    let code = check::check_token_header(&req)?;
    let query: UpdateGoodsCartReq = req.body_json().await?;
    shopping_cart::update_goods_cart(code, id, query)
        .await.map_err(|e| e.into_tide_error(false))?;
    Ok("".into())
}

async fn update_yuyue_goods_cart(mut req: MRequest) -> tide::Result
     {
    let id = check::check_id(&req)?;
    let code = check::check_token_header(&req)?;
    let query: UpdateYuyueGoodsCartReq = req.body_json().await?;
    shopping_cart::update_yuyue_goods_cart(code, id, query)
        .await.map_err(|e| e.into_tide_error(false))?;
    Ok("".into())
}

async fn update_goods_cart_purchase_url(mut req: MRequest) -> tide::Result
     {
    let code = check::check_token_header(&req)?;
    let query: UpdateGoodsCartUrlReq = req.body_json().await?;
    shopping_cart::update_cart_goods_purchase_url(code, query)
        .await.map_err(|e| e.into_tide_error(false))?;
    Ok("".into())
}

async fn delete_goods_cart(mut req: MRequest) -> tide::Result
     {
    let id = check::check_id(&req)?;
    let code = check::check_token_header(&req)?;
    shopping_cart::delete_goods_cart(code, id)
        .await.map_err(|e| e.into_tide_error(false))?;
    Ok("".into())
}

async fn batch_delete_goods_cart(mut req: MRequest) -> tide::Result
     {
    let code = check::check_token_header(&req)?;
    let query: BatchDeleteGoodsCartReq = req.body_json().await?;
    shopping_cart::batch_delete_goods_carts(code, query)
        .await.map_err(|e| e.into_tide_error(false))?;
    Ok("".into())
}

async fn list_goods_cart(mut req: MRequest) -> tide::Result
     {
    let code = check::check_token_header(&req)?;
    let query: KeyWordPageReq = req.query()?;
    let data = shopping_cart::list_goods_carts(code.to_string(), query)
        .await.map_err(|e| e.into_tide_error(false))?;
    Ok(tide::Response::builder(200)
        .content_type(tide::http::mime::JSON)
        .body(serde_json::json!(data))
        .build())
}
