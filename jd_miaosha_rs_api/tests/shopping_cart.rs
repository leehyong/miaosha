use std::time::Duration;
use chrono::Local;
use async_std::task;
use serde_json::{json, Value, from_str, from_slice};
use tide::StatusCode;


#[async_std::test]
async fn test_shopping_cart_api() -> tide::Result<()> {
    let connect_addr = "http://localhost:48180/api/user/activate/code";
    let mut req = surf::post(connect_addr)
        .body(json!(
            {
                "level":4
            }
        ))
        .await?;

    assert_eq!(req.status(), StatusCode::Ok);
    let body = req.body_bytes().await?;
    let v:Value = from_slice(&body)?;
    let code = v["activate_code"].to_string().trim_matches(|c| c == '\"').to_string();
    println!("code: {}, {:?}", code.as_str(), v);
    assert_eq!(code.len(), 32);

    let connect_addr = "http://localhost:48180/api/user/activate";
    let mut req = surf::post(connect_addr)
        .body(json!(
            {
                "activate_code":code.as_str(),
                "mac":"1228A4441A1dffad5B84"
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("api/user/activate done!");

    let connect_addr = "http://localhost:48180/api/shopping_cart";
    let mut req = surf::get(connect_addr)
        .header("token", code.as_str())
        .body("")
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);

    println!("get api/shopping_cart done!");

    let connect_addr = "http://localhost:48180/api/shopping_cart";
    let sku_delete_insert = format!("tet1232_{}", Local::now().timestamp());
    let mut req = surf::post(connect_addr)
        .header("token", code.as_str())
        .body(json!(
            {
                "sku":sku_delete_insert.as_str(),
                "name":"12weqweqeq",
                "purchase_num":2,
                "yuyue_dt":Some(""),
                "is_stock":0,
                "ori_price":"12.23",
                "cur_price":"332.1",
                "purchase_type":"yuyue",
                "purchase_url":"https://divide.jd.com/user_routing?skuId=100016053734&sn=54e0b3fa205eda63b52ebedb96c71af0&from=pc",
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("post api/shopping_cart done!");
    
    let connect_addr = "http://localhost:48180/api/shopping_cart";
    let sku = format!("tet1232_{}", Local::now().timestamp_millis());
    let mut req = surf::post(connect_addr)
        .header("token", code.as_str())
        .body(json!(
            {
                "sku":sku.as_str(),
                "name":"12weqwe22222",
                "yuyue_dt":Some("2021-5-12 12:10"),
                "purchase_num":3,
                "is_stock":1,
                "ori_price":"12.23",
                "cur_price":"332.1",
                "purchase_type":"normal",
                "purchase_url":"https://divide.jd.com/user_routing?skuId=100016053734&sn=54e0b3fa205eda63b52ebedb96c71af0&from=pc",
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("post api/shopping_cart 2222 done!");

    let connect_addr = format!("http://localhost:48180/api/shopping_cart?key_word={}", "12weqwe22");
    let mut req = surf::get(connect_addr.as_str())
        .header("token", code.as_str())
        .body("")
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    let v:Value = req.body_json().await?;
    // let v:Value = from_slice(&body)?;
    assert!(v["records"].is_array() && v["records"].as_array().unwrap().len() >= 1);
    println!("get api/shopping_cart key_word done! ****@@@@**2");

    let connect_addr = "http://localhost:48180/api/shopping_cart";
    let mut req = surf::get(connect_addr)
        .header("token", code.as_str())
        .body("")
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("get api/shopping_cart done! ******2");

    let v:Value = req.body_json().await?;
    println!("carts: {:?}", v);
    let cart_id = v["records"][0]["id"].to_string().parse::<u32>()?;
    println!("cart id: {}!", cart_id);
    let connect_addr = format!("http://localhost:48180/api/shopping_cart/{}", cart_id);
    let mut req = surf::put(connect_addr.as_str())
        .header("token", code.as_str())
        .body(json!(
            {
                "op":1,
                "yuyue_dt":""
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("put api/shopping_cart add done!");

    let mut req = surf::put(connect_addr.as_str())
        .header("token", code.as_str())
        .body(json!(
            {
                "op":2,
                "yuyue_dt":""
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("put api/shopping_cart sub done!");

    let mut req = surf::put(connect_addr.as_str())
        .header("token", code.as_str())
        .body(json!(
            {
                "op":3,
                "yuyue_dt":""
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("put api/shopping_cart immediately done!");

    let mut req = surf::put(connect_addr.as_str())
        .header("token", code.as_str())
        .body(json!(
            {
                "op":4,
                "yuyue_dt":Some("2021-5-16 09:10:11"),
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);

    let mut req = surf::put(connect_addr.as_str())
        .header("token", code.as_str())
        .body(json!(
            {
                "op":9,
                "yuyue_dt":"",
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("put api/shopping_cart OP_OUT_OF_STOCK done!");
    let connect_addr = format!("http://localhost:48180/api/shopping_cart");
    let mut req = surf::put(connect_addr.as_str())
        .header("token", code.as_str())
        .body(json!(
            {
                "sku":sku.as_str(),
                "purchase_url":"http://localhost:48180/api/"
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("put api/shopping_cart purchase_url done!");

    let connect_addr = format!("http://localhost:48180/api/shopping_cart/yuyue/{}", cart_id);
    let mut req = surf::put(connect_addr.as_str())
        .header("token", code.as_str())
        .body(json!(
            {
                "op":4,
                "yuyue_dt":Some("2021-5-21 09:10:11"),
                "yuyue_start_dt":Some("2021-5-18 09:10:11"),
                "yuyue_end_dt":Some("2021-5-26 09:10:11"),
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("put api/shopping_cart purchase_url done!");

    let connect_addr = format!("http://localhost:48180/api/shopping_cart/{}", cart_id);
    let mut req = surf::delete(connect_addr)
        .header("token", code.as_str())
        .body("")
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("delete api/shopping_cart done!");

    let connect_addr = "http://localhost:48180/api/shopping_cart";
    let mut req = surf::delete(connect_addr)
        .header("token", code.as_str())
        .body(json!({
            "ids":vec![cart_id]
        }))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("batch delete api/shopping_cart done!");
    // 重复添加
    let mut req = surf::post(connect_addr)
        .header("token", code.as_str())
        .body(json!(
            {
                "sku":sku_delete_insert.as_str(),
                "name":"12weqweqeq",
                "purchase_num":2,
                "is_stock":1,
                "yuyue_dt":Some(""),
                "ori_price":"12.23",
                "cur_price":"332.1",
                "purchase_type":"seckill",
                "purchase_url":"https://divide.jd.com/user_routing?skuId=100016053734&sn=54e0b3fa205eda63b52ebedb96c71af0&from=pc",
            }
        ))
        .await?;
    assert_eq!(req.status(), StatusCode::Ok);
    println!("post api/shopping_cart done!");

    Ok(())
}