use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use reqwest::{ClientBuilder, Proxy, StatusCode};
use tokio::sync::Mutex;

use crate::*;

use super::default_client;

lazy_static!(
    static ref ACCOUNT_PROXY_IPS:Arc<Mutex<HashMap<IDType, (String, Instant)>>> = Default::default();
);

static TEST_URL: &'static str = "www.baidu.com";

async fn get_account_proxy_ip(account_id: IDType) -> Option<String> {
    let mut lock = ACCOUNT_PROXY_IPS.lock().await;
    if let Some((prxoy, ins)) = lock.get(&account_id) {
        if ins.elapsed().as_secs() < 300 {
            // 5分钟内的代理ip都是有效的
            return Some(prxoy.clone());
        }
        // 超过5分钟的ip需要检测一下代理ip的有效性
        // 只需要代理https的请求
        match Proxy::https(prxoy) {
            Ok(proxy) => {
                let c = default_client("")
                    .proxy(proxy).build().unwrap_or_default();
                match c.get(TEST_URL)
                    .send().await {
                    Ok(resp) => {
                        if resp.status() == StatusCode::OK {
                            return Some(prxoy.clone());
                        }
                    }
                    Err(e) => {
                        error!("代理失效，需要重新获取:{}-{:?}", account_id, e);
                    }
                }
            }
            Err(e) => {
                error!("设置代理ip失败:{}-{:?}", account_id, e);
            }
        }
    }
    return if let Some(ref proxy_ip_pool) = CONFIG.read().await.account_proxy_ip_pool_url() {
        if let Some(proxy_url) = get_available_proxy_url(proxy_ip_pool).await {
            lock.entry(account_id).or_insert((proxy_url.clone(), Instant::now()));
            Some(proxy_url)
        } else {
            warn!("没有获取到代理ip:{}", account_id);
            None
        }
    } else {
        warn!("没有设置ip代理池，无法获取代理ip:{}", account_id);
        None
    };
}


async fn get_available_proxy_url(ip_pool: &str) -> Option<String> {
    let mut proxy_url = String::new();
    // 最多获取 10 次 url
    for times in 1..=10 {
        match reqwest::get(ip_pool).await {
            Ok(r) => {
                // 每次只获取一个ip
                let txt = r.text().await.unwrap_or_default().trim().to_string();
                proxy_url = format!("http://{}", txt);
                break;
            }
            Err(e) => {
                if times == 10 {
                    // 最多获取10次 ip代理
                    error!("Can't get available proxy ip, please check!{}", e);
                    return None;
                }
            }
        }
    }
    if proxy_url.is_empty() {
        None
    } else {
        Some(proxy_url)
    }
}


pub async fn proxy_client_builder(account_id: IDType, sku: &str, cookie: Arc<String>) -> ClientBuilder {
    let mut builder = default_client(cookie.as_str());
    let mut use_proxy = false;
    if let Some(proxy_url) = get_account_proxy_ip(account_id).await {
        if let Ok(_proxy) = reqwest::Proxy::all(&proxy_url) {
            use_proxy = true;
            builder = builder.proxy(_proxy);
            info!("成功设置https请求代理:{}-{},{}!", account_id, sku, proxy_url);
        }
    }
    if !use_proxy {
        warn!("没有使用https请求代理:{}-{}!", account_id, sku);
    }
    builder
}