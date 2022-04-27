#![windows_subsystem = "windows"]
#![allow(non_snake_case)]
#[macro_use]
extern crate lazy_static;

use std::env;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use chrono::{Date, DateTime, FixedOffset, Local, TimeZone};
use log::{debug, error, info, warn};
use serde::Deserialize;
use tokio::sync::RwLock;

use platform::Platform;


pub mod error;
pub mod models;
pub mod platform;
pub mod services;
mod ui;
pub mod utils;
pub mod types;
pub mod executor;

lazy_static! {
    pub static ref CONFIG: Arc<RwLock<Config>> = Arc::new(RwLock::new(Config::default()));
}
pub type PKLocal = Local;
pub type PKDate = Date<PKLocal>;
pub type PKDateTime = DateTime<PKLocal>;
pub type IDType = u32;
pub type PurchaseType = String;
pub static YUYUE:&'static str = "yuyue";    // 预约
pub static PRESALE:&'static str = "preSale";    // 预售，定金模式
pub static NORMAL:&'static str = "normal";  // 普通抢购
pub static SECOND_KILL:&'static str = "secKill";  // 秒杀

const PROXY_IP_POOL:&'static str = "http://webapi.http.zhimacangku.com/getip?num=1&type=1&pro=&city=0&yys=0&port=1&time=1&ts=0&ys=0&cs=0&lb=1&sb=0&pb=4&mr=1&regions=";
const HOME: &'static str = "HOME";

#[derive(Deserialize, Default)]
pub struct Config {
    addr: String,
    proxy_ip_pool_url: Option<String>,
}

impl Config {
    pub fn server_addr(&self) -> String {
        self.addr.clone()
    }

    pub fn account_proxy_ip_pool_url(&self) -> Option<String> {
        self.proxy_ip_pool_url.clone()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let exe_dir = env::current_exe()?.parent().unwrap().to_owned();
    let home = exe_dir.to_str().unwrap();
    env::set_var(HOME, home);
    let log_yaml = exe_dir.join("config/log.yml");
    info!("目录:{}, \n日志配置文件:{}", home, log_yaml.display());
    log4rs::init_file(log_yaml, Default::default()).unwrap();
    ui::run_app(exe_dir);
    Ok(())
}
