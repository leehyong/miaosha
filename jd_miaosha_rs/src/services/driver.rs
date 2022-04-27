use std::env;
use std::fmt::Error;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use log::{debug, error, info, warn};
use serde_json::json;
use thirtyfour::{DesiredCapabilities, GenericWebDriver, support, TimeoutConfiguration, WebDriver, WebDriverCommands};
use tokio::sync::RwLock;

use crate::error::{OpError, Result};
use crate::platform::Platform;

pub use super::reqwest_async::build_driver_url;
use super::reqwest_async::MiaoshaReqwestDriverAsync;

/// copy from https://github.com/Mubelotix/webdriver_process/blob/master/src/session.rs
pub struct DriverManager {
    webdriver_process: Option<std::process::Child>,
}

lazy_static! {
    pub static ref CHROME_HEALESS_DRIVER: Pin<Arc<RwLock<Option<WebDriver>>>> =
        Arc::pin(RwLock::new(None));
}

impl Default for DriverManager {
    fn default() -> Self {
        Self {
            webdriver_process: None,
        }
    }
}

impl DriverManager {
    pub const PORT: &'static str = "34441";

    pub async fn init_servers() -> Result<Self> {
        let platform = Platform::current();
        if let Platform::Unknown = platform {
            panic!("Unsupported platform");
        }
        // chromedriver_mac64 路径需要自定义
        let home = PathBuf::from(env::var(crate::HOME).unwrap());
        let driver_path = home.join(if platform == Platform::Windows {
            "assets/driver/chromedriver.exe"
        } else if platform == Platform::MacOS {
            "assets/driver/chromedriver_mac64"
        } else {
            "assets/driver/chromedriver_linux64"
        });
        info!("driver_path: {}", driver_path.to_str().unwrap());
        let p = Command::new(driver_path.to_str().unwrap())
            .arg(format!("--port={}", Self::PORT))
            .arg("--readable-timestamp")
            .arg("--log-level=INFO")
            .arg(format!("--log-path={}", home.join("log/driver.log").to_str().unwrap()))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        tokio::fs::write(
            home.join("pid.txt").as_path(),
            p.id().to_string().as_bytes(),
        )
        .await?;
        info! {"DriverManager created successfully. {}", p.id()}
        return Ok(Self {
            webdriver_process: Some(p),
        });
    }

    pub async fn init_driver_clients() -> Result<()> {
        info!("initing driver clients 1");
        {
            if CHROME_HEALESS_DRIVER.read().await.is_none() {
                let mut guard = CHROME_HEALESS_DRIVER.write().await;
                let driver = init_driver(true).await?;
                *guard = Some(driver)
            }
        }
        info! {"Chrome drivers inited successfully. " }
        Ok(())
    }
}

impl Drop for DriverManager {
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        if self.webdriver_process.is_some() {
            warn!("Killing webdriver_process process (may fail silently)");
            self.webdriver_process.take().unwrap().kill();
        }
    }
}

pub async fn init_driver(headless: bool) -> Result<WebDriver> {
    let mut caps = DesiredCapabilities::chrome();
    if headless {
        caps.set_headless()?;
        caps.add_chrome_arg("blink-settings=imagesEnabled=false")?;
    } else {
        // 启动时不打开启动窗口
        // 或者文件： chrome_switches.cpp
        // 参见 https://chromium.googlesource.com/chromium/src/+/refs/heads/main/chrome/common/chrome_switches.cc
        // caps.add_chrome_arg("no-startup-window")?;
        caps.add_chrome_arg("--disable-gpu")?;
    }
    let driver = WebDriver::new_with_timeout(
        format!("http://localhost:{}", DriverManager::PORT).as_str(),
        &caps,
        Some(Duration::new(300, 0)),
    )
        .await?;
    let set_timeouts = TimeoutConfiguration::new(
        Some(Duration::new(300, 0)),
        Some(Duration::new(300, 0)),
        Some(Duration::new(300, 0)),
    );
    driver.set_timeouts(set_timeouts.clone()).await?;
    Ok(driver)
}


pub type MiasoshaWebDriver = GenericWebDriver<MiaoshaReqwestDriverAsync>;


pub async fn init_miaosha_driver(headless: bool) -> Result<MiasoshaWebDriver> {
    let mut caps = DesiredCapabilities::chrome();
    if headless {
        caps.set_headless()?;
        caps.add_chrome_arg("blink-settings=imagesEnabled=false")?;
    } else {
        // 启动时不打开启动窗口
        // 或者文件： chrome_switches.cpp
        // 参见 https://chromium.googlesource.com/chromium/src/+/refs/heads/main/chrome/common/chrome_switches.cc
        // caps.add_chrome_arg("no-startup-window")?;
        caps.add_chrome_arg("--disable-images")?;
        caps.add_chrome_arg("--disable-gpu")?;
    }

    let mut driver = MiasoshaWebDriver::new_with_timeout(
        format!("http://localhost:{}", DriverManager::PORT).as_str(),
        &caps,
        Some(Duration::new(300, 0)),
    )
        .await?;
    let set_timeouts = TimeoutConfiguration::new(
        Some(Duration::new(300, 0)),
        Some(Duration::new(300, 0)),
        Some(Duration::new(300, 0)),
    );
    driver.set_timeouts(set_timeouts.clone()).await?;
    Ok(driver)
}