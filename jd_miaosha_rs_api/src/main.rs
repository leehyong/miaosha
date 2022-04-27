use std::env;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate rbatis;

use chrono::{Local, Date, DateTime, FixedOffset};
use log::{info, warn, error};
use std::sync::Arc;
use async_std::fs;
use async_std::sync::RwLock;
use serde::{Deserialize, Serializer, Deserializer};
use rbatis::rbatis::{Rbatis};
use rbatis::core::db::DBPoolOptions;

pub mod utils;
pub mod access;
pub mod models;
pub mod apis;
pub mod error;
pub mod req;
pub mod services;
mod types;

use crate::access::{Permission, Role};
use crate::utils::token::Claims;

use types::*;

pub type PKLocal = Local;
pub type PKDate = Date<PKLocal>;
pub type PKDateTime = DateTime<PKLocal>;
pub type IDType = u32;
type AppState = Arc<RwLock<State>>;
pub type MRequest = tide::Request<AppState>;

const HOME: &'static str = "HOME";
lazy_static!(
    static ref REDIS_CLIENT : Arc<RwLock<Option<redis::Client>>> = Arc::new(RwLock::new(None));
    static ref DB_CLIENT : Rbatis = Rbatis::new();
);


#[derive(Deserialize)]
struct Config {
    ip: String,
    port: u16,
    workers: u8,
    redis_uri: String,
    db_uri: String,
    white_mac_list: Vec<String>,
    vip_level_users: Vec<u32>,
}

pub struct State {
    pub white_mac_list: Vec<String>,
    pub vip_level_users: Vec<u32>,
}

async fn init() -> Config {
    let exe_dir = env::current_exe().unwrap().parent().unwrap().to_owned();
    let home = exe_dir.to_str().unwrap();
    env::set_var(HOME, home);
    let log_yaml = exe_dir.join("config/log.yml");
    info!("目录:{}, \n日志配置文件:{}", home, log_yaml.display());
    let conf_file = exe_dir.join("config/conf.toml");
    let config: Config = toml::from_slice(
        fs::read(conf_file).await.unwrap().as_slice()).unwrap();
    if (*REDIS_CLIENT.read().await).is_none() {
        let mut wc = REDIS_CLIENT.write().await;
        *wc = Some(redis::Client::open(config.redis_uri.as_str()).unwrap());
    }
    let mut opts = DBPoolOptions::default();
    opts.max_connections = 50;
    DB_CLIENT.link_opt(config.db_uri.as_str(), &opts).await.unwrap();
    // log4rs::init_file(log_yaml, Default::default()).unwrap();
    config
}

async fn get_redis_conn() -> redis::RedisResult<redis::aio::MultiplexedConnection> {
    let client = REDIS_CLIENT.read().await;
    if let Some(ref _client) = *client {
        _client.get_multiplexed_async_connection().await
    } else {
        Err((redis::ErrorKind::ClientError, "redis client 初始化失败").into())
    }
}

pub async fn start_server() -> tide::Result<()> {
    let config = init().await;
    let bind_addr = format!("{}:{}", config.ip.as_str(), config.port);
    // tide 的 femme 不能跟log4rs集成，故暂时去掉以下的配置
    tide::log::with_level(tide::log::LevelFilter::Debug);
    // 把 - 替换为 :
    let state = Arc::new(RwLock::new(
        State {
            white_mac_list: config
                .white_mac_list
                .iter()
                .map(|mac| mac.to_uppercase().replace("-", ":"))
                .collect(),
            vip_level_users: config.vip_level_users,
        }));
    let mut app = tide::with_state(state);
    app.with(tide::log::LogMiddleware::new());
    apis::user::api(&mut app);
    apis::shopping_cart::api(&mut app);
    app.listen(bind_addr.as_str()).await?;
    Ok(())
}


#[async_std::main]
async fn main() -> tide::Result<()> {
    start_server().await
}

