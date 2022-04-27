use std::time::Duration;
use chrono::Local;
use async_std::task;
use serde_json::{json, Value, from_str, from_slice};
use tide::StatusCode;


#[async_std::test]
async fn test_user_api() -> tide::Result<()> {
    let connect_addr = "http://localhost:48180/api/user/activate/code";
    let mut req = surf::post(connect_addr)
        .body(json!(
            {
                "level":3
            }
        ))
        .await?;

    assert_eq!(req.status(), StatusCode::Ok);
    let body = req.body_bytes().await?;
    let v: Value = from_slice(&body)?;
    let code = v["activate_code"].to_string().trim_matches(|c| c == '\"').to_string();
    println!("code: {}, {:?}", code.as_str(), v);
    assert_eq!(code.len(), 32);

    let connect_addr = "http://localhost:48180/api/user/activate";
    let mut req = surf::post(connect_addr)
        .body(json!(
            {
                "activate_code":code.as_str(),
                "mac":"1228A4441A1dffad5B"
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("api/user/activate done!");

    let connect_addr = "http://localhost:48180/api/user";
    let mut req = surf::get(connect_addr)
        .header("token", code.as_str())
        .body("")
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);

    let connect_addr = "http://localhost:48180/api/account";
    let mut req = surf::get(connect_addr)
        .header("token", code.as_str())
        .body("")
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);

    println!("get api/account done!");

    let connect_addr = "http://localhost:48180/api/account";
    let mut req = surf::post(connect_addr)
        .header("token", code.as_str())
        .body(json!(
            {
                "account":format!("tet1232_{}", Local::now().timestamp()),
                "pwd":"1228Aasdqweqweqeq",
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("post api/account done!");

    let connect_addr = "http://localhost:48180/api/account";
    let mut req = surf::get(connect_addr)
        .header("token", code.as_str())
        .body("")
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("get api/account done! ******2");

    let body = req.body_bytes().await?;
    let v: Value = from_slice(&body)?;
    println!("accounts: {:?}", v);
    let account_id = v["records"][0]["id"].to_string().parse::<u32>()?;
    println!("account id: {}!", account_id);
    let connect_addr = format!("http://localhost:48180/api/account/{}", account_id);
    let mut req = surf::put(connect_addr)
        .header("token", code.as_str())
        .body(json!(
            {
                "account":format!("tet1232_{}", Local::now().timestamp()),
                "pwd":"1228Aasdeqweqeq",
                "cookie":"rwerwer43111llkoxc;etert;weq1231;444ada1",
                "eid":"fafa",
                "fp":"nfaffzz",
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("put api/account done!");

    let connect_addr = format!("http://localhost:48180/api/account/{}", account_id);
    let mut req = surf::delete(connect_addr)
        .header("token", code.as_str())
        .body("")
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("delete api/account done!");

    let connect_addr = "http://localhost:48180/api/account";
    let mut req = surf::delete(connect_addr)
        .header("token", code.as_str())
        .body(json!({
            "ids":vec![account_id]
        }))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("batch delete api/account done!");
    Ok(())
}

#[async_std::test]
async fn test_user_api2() -> tide::Result<()> {
    let connect_addr = "http://localhost:48180/api/user/activate/code";
    let mut req = surf::post(connect_addr)
        .body(json!(
            {
                "level":3
            }
        ))
        .await?;

    assert_eq!(req.status(), StatusCode::Ok);
    let body = req.body_bytes().await?;
    let v: Value = from_slice(&body)?;
    let code = v["activate_code"].to_string().trim_matches(|c| c == '\"').to_string();
    println!("code: {}, {:?}", code.as_str(), v);
    assert_eq!(code.len(), 32);

    let expired_codes = vec![
        code.as_str(),
        "3C2CFA9494007B71598D82DA6D3FC346",
        "D9CF51FB1F35093CCE61FF630F32E66D",
        "ACF9F36DE722CE7884DB24E7DC0C3F7F"
    ];
    let connect_addr = "http://localhost:48180/api/user/refresh";
    let mut req = surf::post(connect_addr)
        .body(json!(
            {
                "codes":expired_codes
            }
        ))
        .await?;

    assert_eq!(req.status(), StatusCode::Ok);
    Ok(())
}


#[async_std::test]
async fn test_user_heartbeat() -> tide::Result<()> {
    let connect_addr = "http://localhost:48180/api/user/activate/code";
    let mut req = surf::post(connect_addr)
        .body(json!(
            {
                "level":3
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    let body = req.body_bytes().await?;
    let v: Value = from_slice(&body)?;
    let code = v["activate_code"].to_string().trim_matches(|c| c == '\"').to_string();
    println!("code: {}, {:?}", code.as_str(), v);
    assert_eq!(code.len(), 32);
    let mac = "1228A4441A1dffad5Bdasdad";
    let connect_addr = "http://localhost:48180/api/user/activate";
    let mut req = surf::post(connect_addr)
        .body(json!(
            {
                "activate_code":&code,
                "mac":&mac
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("api/user/activate done!");

    let connect_addr = "http://localhost:48180/api/user/heartbeat";
    let mut req = surf::post(connect_addr)
        .header("token", &code)
        .body(json!(
            {
                "mac":&mac
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("api/user/heartbeat 1 done");
    let mac_403 = "1228A4441A1dffasdad22";
    let mut req = surf::post(connect_addr)
        .header("token", &code)
        .body(json!(
            {
                "mac":mac_403
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Unauthorized);
    println!("api/user/heartbeat 2 done");
    Ok(())
}