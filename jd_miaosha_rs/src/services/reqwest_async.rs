use std::fmt::Debug;
use std::time::Duration;
use async_trait::async_trait;
use reqwest::{
    self,
    header::{HeaderMap, ACCEPT, AUTHORIZATION, CONNECTION, CONTENT_TYPE, USER_AGENT},
};

use thirtyfour::http::connection_async::WebDriverHttpClientAsync;
use thirtyfour::{
    error::{WebDriverError, WebDriverResult},
    RequestData, RequestMethod,
};
use crate::utils::get_useragent;
use super::SESSION_COOKIES;

/// Asynchronous http to the remote WebDriver server.
#[derive(Debug)]
pub struct MiaoshaReqwestDriverAsync {
    url: String,
    pub client: reqwest::Client,
    timeout: Duration,
}

fn build_reqwest_headers() -> WebDriverResult<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, "application/json".parse().unwrap());
    headers.insert(CONTENT_TYPE, "application/json;charset=UTF-8".parse().unwrap());
    headers.insert(USER_AGENT, get_useragent().parse().unwrap());
    headers.insert(CONNECTION, "keep-alive".parse().unwrap());
    Ok(headers)
}

static DELIMITER:&'static str = "::::";

pub fn build_driver_url(url:String, ck:&str) -> String{
        format!("{}{}{}", url,DELIMITER, ck)
}

#[async_trait]
impl WebDriverHttpClientAsync for MiaoshaReqwestDriverAsync {
    fn create(remote_server_addr: &str) -> WebDriverResult<Self> {
        let headers = build_reqwest_headers()?;
        Ok(MiaoshaReqwestDriverAsync {
            url: remote_server_addr.trim_end_matches('/').to_owned(),
            client: reqwest::Client::builder().default_headers(headers).build()?,
            timeout: Duration::from_secs(300),
        })
    }

    fn set_request_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Execute the specified command and return the data as serde_json::Value.
    async fn execute(&self, request_data: RequestData) -> WebDriverResult<serde_json::Value> {
        let url = self.url.clone() + &request_data.url;
        let mut request = match request_data.method {
            RequestMethod::Get => self.client.get(&url),
            RequestMethod::Post => self.client.post(&url),
            RequestMethod::Delete => self.client.delete(&url),
        };
        request = request.timeout(self.timeout);

        if let Some(mut x) = request_data.body {
            log::info!("x:{}", x.to_string());
            if let Some(u) = x.get("url"){
                let url2 = u.as_str().unwrap_or_default();
                let mut ck= "";
                let url3;
                if let Some((_url, _ck)) = url2.split_once(DELIMITER){
                    ck = _ck;
                    url3 = _url;
                }else{
                    url3 = url.as_str();
                }
                log::info!("{}, cookie:{}", url2, ck);
                if !ck.is_empty(){
                    request = request.header("cookie", ck);
                }
                x["url"] = serde_json::json!(url3);
            }
            request = request.json(&x);
        }
        let resp = request.send().await?;

        match resp.status().as_u16() {
            200..=399 => Ok(resp.json().await?),
            400..=599 => {
                let status = resp.status().as_u16();
                Err(WebDriverError::parse(status, resp.text().await?))
            }
            _ => unreachable!(),
        }
    }
}
