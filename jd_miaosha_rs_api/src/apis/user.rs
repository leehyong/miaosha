use tide::Server;
use tide::StatusCode;
use log::error;
use validator::Validate;
use mac_address::get_mac_address;
use crate::models::User;
use crate::req::*;
use crate::utils::{token, check};
use crate::services::user;
use crate::error::*;
use crate::*;


pub fn api(app: &mut Server<AppState>)
{
    let mut user_service = tide::with_state(app.state().clone());
    user_service
        .at("/activate").post(activate)
        .at("/code").post(create_activate_code);
    let mut account_service = tide::with_state(app.state().clone());
    account_service
        .at("/:id")
        .put(update_account)
        .delete(delete_account);
    app.at("/api/user")
        .get(user_info)
        .nest(user_service)
        .at("/refresh").post(refresh);
    app.at("/api/account")
        .get(list_accounts)
        .post(add_account)
        .delete(batch_delete_accounts)
        .nest(account_service);
    app.at("/api/user/heartbeat").post(heartbeat);
}


async fn create_activate_code(mut req: MRequest) -> tide::Result
{
    // 获取激活码， 需要在特定的机器上才能操作
    match get_mac_address() {
        Ok(m) => {
            match m {
                Some(_mac) => {
                    let mac = format!("{}", _mac).to_uppercase().replace("-", ":");
                    let pass = {
                        let state = req.state().read().await;
                        state.white_mac_list.contains(&mac)
                    };
                    if pass {
                        let CreateActivateCodeReq { level } = req.body_json().await?;
                        let code = user::create_user_activate_code(level).await.map_err(
                            |e| e.into_tide_error(false))?;
                        return Ok(tide::Response::builder(StatusCode::Ok)
                            .content_type(tide::http::mime::JSON)
                            .body(code)
                            .build());
                    }
                }
                None =>{
                    error!("Can't find any mac addr.");
                }
            }
        }
        Err(e) => {
            error!("mac error: {}", e.to_string());
        }
    }
    return Ok(tide::Response::builder(StatusCode::Unauthorized)
        .content_type(tide::http::mime::PLAIN)
        .body("")
        .build());
}

async fn activate(mut req: MRequest) -> tide::Result
{
    let req: ActivateReq = req.body_json().await?;
    user::activate(req).await.map_err(|e| e.into_tide_error(true))?;
    Ok("".into())
}

async fn refresh(mut req: MRequest) -> tide::Result
{
    let req: RefreshReq = req.body_json().await?;
    user::refresh(req).await.map_err(|e| e.into_tide_error(true))?;
    Ok("".into())
}


async fn add_account(mut req: MRequest) -> tide::Result
{
    let code = check::check_token_header(&req)?;
    let query: AddAccountReq = req.body_json().await?;
    let state = req.state().read().await;
    user::add_account(code, query, &state.vip_level_users)
        .await.map_err(|e| e.into_tide_error(false))?;
    Ok("".into())
}

async fn update_account(mut req: MRequest) -> tide::Result
{
    let id = check::check_id(&req)?;
    let code = check::check_token_header(&req)?;
    let query: UpdateAccountReq = req.body_json().await?;
    user::update_account(code, id, query)
        .await.map_err(|e| e.into_tide_error(false))?;
    Ok("".into())
}

async fn delete_account(mut req: MRequest) -> tide::Result
{
    let id = check::check_id(&req)?;
    let code = check::check_token_header(&req)?;
    user::delete_account(code, id)
        .await.map_err(|e| e.into_tide_error(false))?;
    Ok("".into())
}

async fn batch_delete_accounts(mut req: MRequest) -> tide::Result
{
    let code = check::check_token_header(&req)?;
    let query: BatchDeleteAccountReq = req.body_json().await?;
    user::batch_delete_accounts(code, query)
        .await.map_err(|e| e.into_tide_error(false))?;
    Ok("".into())
}

async fn list_accounts(mut req: MRequest) -> tide::Result
{
    let code = check::check_token_header(&req)?;
    let query: KeyWordPageReq = req.query()?;
    let data = user::list_accounts(code.to_string(), query)
        .await.map_err(|e| e.into_tide_error(false))?;
    Ok(tide::Response::builder(200)
        .content_type(tide::http::mime::JSON)
        .body(serde_json::json!(data))
        .build())
}

async fn user_info(mut req: MRequest) -> tide::Result{
    let code = check::check_token_header(&req)?;
    let data = user::user_info(code).await?;
    Ok(tide::Response::builder(200)
        .content_type(tide::http::mime::JSON)
        .body(serde_json::json!(data))
        .build())
}

async fn heartbeat(mut req: MRequest) -> tide::Result{
    let code = check::check_token_header(&req)?;
    let data: HeartbeatReq = req.body_json().await?;
    user::heartbeat(code, data)
        .await
        .map_err(|e| e.into_tide_error(false))?;
    Ok("".into())
}