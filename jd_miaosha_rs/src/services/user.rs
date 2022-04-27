use crate::error::{JdMiaoshaError, OpError, Result};
use crate::models::*;
use crate::services::driver::*;
use crate::ui::UserCookieInfo;
use crate::utils::sleep;
use crate::*;
use futures::join;
use http::{Method, StatusCode};
use log::{debug, error, info, warn};
use rand::prelude::*;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, ClientBuilder, Error as ReqwestError, Request,
};
use scraper::{Html, Selector};
use serde::Deserialize;
use serde_json::{from_slice, from_str, json, to_string, Value};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use thirtyfour::error::WebDriverError;
use thirtyfour::prelude::*;
use thirtyfour::GenericWebDriver;
use thirtyfour::{By, WebDriverCommands};
use tokio::fs::{create_dir_all, File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::RwLock;

lazy_static! {
    static ref DRIVER: Arc<RwLock<Option<DriverManager>>> = Default::default();
}

#[derive(Deserialize)]
struct UserData {
    pub account: String,
    pub pwd: String,
    pub cookie: String,
    pub eid: String,
    pub fp: String,
    pub cookie_last_update_dt: Option<String>,
}

#[derive(Clone)]
pub struct UserService;

impl UserService {
    const LOGIN_URL: &'static str = "https://passport.jd.com/new/login.aspx";
    const CHECK_COOKIE_URL: &'static str =
        "https://order.jd.com/center/list.action?s=1024&search=0&d=1";

    pub async fn init_web_driver() -> Result<()> {
        let exe_dir = PathBuf::from(env::var(HOME).unwrap());
        let conf_file = exe_dir.join("config/conf.toml");
        let config: Config =
            toml::from_slice(tokio::fs::read(conf_file).await.unwrap().as_slice()).unwrap();
        {
            let mut guard = CONFIG.write().await;
            *guard = config;
        }
        // 初始化 chromedriver
        // 需要把 DriverManager 用全局变量保存起来，不然会被 drop，而后 kill 对应的 chromedriver进程
        let dm = DriverManager::init_servers().await?;
        let mut lock = DRIVER.write().await;
        if lock.is_none() {
            *lock = Some(dm);
            info!("init_web_driver!");
        }
        Ok(())
    }

    pub async fn init_driver_clients() -> Result<()> {
        sleep(300).await;
        Ok(())
        // match DriverManager::init_driver_clients().await{
        //     Ok(_) => Ok(()),
        //     Err(e) =>{
        //         error!("{:?}", e);
        //         Err(e)
        //     }
        // }
    }

    pub async fn get_cookie_eid_fp(
        id: IDType,
        account: &str,
        pwd: &str,
    ) -> Result<Option<UserCookieInfo>> {
        // if CHROME_DRIVER.read().await.is_none(){
        //     let mut guard = CHROME_DRIVER.write().await;
        //     let driver = init_driver(false).await?;
        //     *guard = Some(driver)
        // }
        // let driver = CHROME_DRIVER.read().await;
        let mut driver = init_driver(false).await?;
        driver.get(Self::LOGIN_URL).await?;
        let mut final_cookies_str = String::with_capacity(1200);
        let mut eid = String::with_capacity(100);
        let mut fp_str = String::with_capacity(100);
        // 循环加载登录, 直到成功获取cookie，eid，fp
        // 等待页面加载完成, 时间太短了，页面加载不了
        let mut times = 1;
        loop {
            match driver
                .find_element(By::XPath(r#"//div[@class="login-tab login-tab-r"]//a"#))
                .await
            {
                Ok(div_element) => {
                    div_element.click().await?;
                    break;
                }
                _ => {
                    sleep(300).await;
                    times += 1;
                    if times > 10 {
                        // 超过30次重试就放弃了
                        error!(
                            "用户[{},{}...]登录失败超过30次了，已放弃!",
                            &account,
                            &pwd.split_at(4).0
                        );
                        driver.quit().await?;
                        return Ok(None); // todo: return error
                    }
                }
            }
        }
        let elem_form = driver.find_element(By::Id("formlogin")).await?;

        loop {
            // <input type="hidden" name="fp" id="sessionId" value="086e4713a310c7f666b7d13b7a05257d" class="hide">
            eid = elem_form
                .find_element(By::Id("eid"))
                .await?
                .value()
                .await?
                .unwrap();
            if eid.is_empty() {
                sleep(50).await;
                continue;
            }
            fp_str = elem_form
                .find_element(By::Id("sessionId"))
                .await?
                .value()
                .await?
                .unwrap();

            break;
        }

        let input_name = elem_form.find_element(By::Id("loginname")).await?;
        // Find element from element.
        input_name.send_keys(account).await?;
        let input_pwd = elem_form.find_element(By::Id("nloginpwd")).await?;
        input_pwd.send_keys(pwd).await?;
        let click_login = elem_form.find_element(By::Css("a#loginsubmit")).await?;
        click_login.click().await?;
        times = 1;
        while let Err(e) = driver.find_element(By::Css("li#ttbar-login")).await {
            match e {
                WebDriverError::NotFound(_, _) => {
                    // 等待50毫秒
                    sleep(50).await
                }
                WebDriverError::NoSuchWindow(e)
                | WebDriverError::NoSuchAlert(e)
                | WebDriverError::NoSuchFrame(e)
                | WebDriverError::NoSuchElement(e) => {
                    // 窗口被关闭了
                    error!(" login error, account:{},  error:{:?}", account, e);
                    driver.quit().await?;
                    return Ok(None);
                    // 重复，直到找到用户别名
                }
                e2 @ _ => {
                    error!(" login error, account:{},  error:{:?}", account, e2);
                    sleep(100).await;
                    if times > 600 {
                        driver.quit().await?;
                        return Ok(None);
                    }
                    times += 1;
                }
            }
        }
        driver.get("https://order.jd.com/center/list.action").await?;
        let cookies = driver.session.get_cookies().await?;
        final_cookies_str = cookies
            .iter()
            .map(|c| format!("{}={}", c.name(), c.value().as_str().unwrap_or("")))
            .collect::<Vec<String>>()
            .join(";");
        // 手动关闭它， 不然程序卡在这里
        driver.quit().await?;
        info!(
            "done!  --- eid:{} fp:{},\n cookie[30]:{}",
            &eid,
            &fp_str,
            &final_cookies_str[..=30]
        );
        return Ok(Some(UserCookieInfo {
            id,
            eid,
            fp: fp_str,
            cookie: Arc::new(final_cookies_str),
            account: account.to_string(),
        }));
    }

    pub async fn is_valid_cookie(account: String, cookie: Arc<String>) -> Result<(String, bool)> {
        // 禁止重定向
        let resp = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?
            .get(Self::CHECK_COOKIE_URL)
            .header("cookie", cookie.as_str())
            .header("authority", "order.jd.com")
            .header("cache-control", "max-age=0")
            .header("sec-ch-ua", r#"" Not A;Brand";v="99", "Chromium";v="90", "Google Chrome";v="90""#)
            .header("sec-ch-ua-mobile", "?0")
            .header("dnt", "1")
            .header("upgrade-insecure-requests", "1")
            .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/90.0.4430.72 Safari/537.36")
            .header("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9")
            .header("sec-fetch-site", "same-site")
            .header("sec-fetch-mode", "navigate")
            .header("sec-fetch-user", "?1")
            .header("sec-fetch-dest", "document")
            .header("referer", "https://item.jd.com/")
            .header("accept-language", "zh-CN,zh;q=0.9")
            .send()
            .await?;
        let status = resp.status();
        // cookie失效的账号，会被重定向到登录页
        info!("account:{} \t status: {:?} ", account.as_str(), status);
        Ok((account, status == StatusCode::OK))
    }

    pub async fn list_accounts(
        code: String,
        key_word: String,
        page_no: i64,
    ) -> Result<Option<AccountsPageState>> {
        let addr_prefix = CONFIG.read().await.server_addr();
        let url = format!(
            "{}/api/account?page_no={}&key_word={}",
            addr_prefix.as_str(),
            page_no,
            key_word
        );
        let resp = reqwest::Client::new()
            .get(url)
            .header("token", code.as_str())
            .send()
            .await?;
        if resp.status() != StatusCode::OK {
            error!("code:{}, {}", code, resp.status().as_str());
            return Ok(None);
        }
        let records: AccountsPageState = serde_json::from_value(resp.json().await?)?;
        Ok(Some(records))
    }

    fn user_data_file_path() -> PathBuf {
        let mut home = PathBuf::from(env::var(crate::HOME).unwrap());
        let db_file_path = home.join("db/user.json");
        info!("db_file_path: {:?}", db_file_path);
        db_file_path
    }

    fn user_code_file_path() -> PathBuf {
        let mut home = PathBuf::from(env::var(crate::HOME).unwrap());
        let db_file_path = home.join("db/code.json");
        info!("{:?}", db_file_path);
        db_file_path
    }

    pub async fn store_user_data(data: Vec<UserState>) -> Result<()> {
        let db_file_path = Self::user_data_file_path();
        let mut db_file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .append(false)
            .open(db_file_path)
            .await?;
        let d = to_string(&data)?;
        db_file.write_all(d.as_bytes()).await?;
        Ok(())
    }

    pub async fn load_user_activate_info() -> Result<Option<UserInfo>> {
        let db_file_path = Self::user_code_file_path();
        match File::open(&db_file_path).await {
            Ok(db_file) => {
                let mut reader = BufReader::new(db_file);
                let mut fd = vec![];
                let _ = reader.read_to_end(&mut fd).await?;
                Ok(serde_json::from_slice(fd.as_slice())?)
            }
            Err(e) => {
                use std::io::ErrorKind::NotFound;
                if e.kind() == NotFound {
                    let parent = db_file_path.parent().unwrap();
                    create_dir_all(parent).await?;
                    File::create(db_file_path.as_path()).await?;
                    return Ok(None);
                }
                return Err(e.into());
            }
        }
    }

    pub async fn store_user_activate_info(user_info: String) -> Result<()> {
        let db_file_path = Self::user_code_file_path();
        let mut db_file = OpenOptions::new()
            .write(true)
            .append(false)
            .truncate(true)
            .open(db_file_path)
            .await?;
        db_file.write_all(user_info.as_bytes()).await?;
        Ok(())
    }

    pub async fn activate(code: String) -> Result<Option<UserInfo>> {
        let addr_prefix = CONFIG.read().await.server_addr();
        let url = format!("{}/api/user/activate", addr_prefix.as_str());
        let resp = reqwest::Client::new()
            .post(url)
            .body(
                json!({
                    "mac":UserInfo::get_user_mac_address(),
                    "activate_code":code.as_str()
                })
                .to_string(),
            )
            .send()
            .await?;
        if resp.status() != StatusCode::OK {
            error!("code:{}, {}", code.as_str(), resp.status().as_str());
            return Ok(None);
        }
        let u = Self::user_info(code).await?;
        Ok(u)
    }

    pub async fn user_info(code: String) -> Result<Option<UserInfo>> {
        let addr_prefix = CONFIG.read().await.server_addr();
        let url = format!("{}/api/user", addr_prefix.as_str());
        let resp = reqwest::Client::new()
            .get(url)
            .header("token", code.as_str())
            .send()
            .await?;
        if resp.status() != StatusCode::OK {
            error!("code:{}, {}", code, resp.status().as_str());
            return Ok(None);
        }
        let u: UserInfo = resp.json().await?;
        info!("user_info: {:?}", u);
        Ok(Some(u))
    }

    pub async fn delete_accounts(code: String, ids: Option<Vec<IDType>>) -> Result<()> {
        let addr_prefix = CONFIG.read().await.server_addr();
        let url = format!("{}/api/account", addr_prefix.as_str());
        let body;
        if let Some(ids) = ids {
            body = json!({ "ids": ids })
        } else {
            // 删除全部账号
            body = json!({});
        }
        let resp = reqwest::Client::new()
            .delete(url)
            .header("token", code.as_str())
            .body(body.to_string())
            .send()
            .await?;
        if resp.status() != StatusCode::OK {
            error!("code:{}, {}", code, resp.status().as_str());
        }
        Ok(())
    }

    pub async fn create_account(code: String, body: String) -> Result<()> {
        let addr_prefix = CONFIG.read().await.server_addr();
        let url = format!("{}/api/account", addr_prefix.as_str());
        let resp = reqwest::Client::new()
            .post(url)
            .header("token", code.as_str())
            .body(body)
            .send()
            .await?;
        if resp.status() != StatusCode::OK {
            error!("code:{}, {}", code, resp.status().as_str());
        }
        Ok(())
    }

    pub async fn update_account(code: String, id: IDType, body: String) -> Result<()> {
        let addr_prefix = CONFIG.read().await.server_addr();
        let url = format!("{}/api/account/{}", addr_prefix.as_str(), id);
        let resp = reqwest::Client::new()
            .put(url)
            .header("token", code.as_str())
            .body(body)
            .send()
            .await?;
        if resp.status() != StatusCode::OK {
            error!("code:{}, {}", code, resp.status().as_str());
        }
        Ok(())
    }

    pub async fn delete_account(code: String, id: IDType) -> Result<()> {
        let addr_prefix = CONFIG.read().await.server_addr();
        let url = format!("{}/api/account/{}", addr_prefix.as_str(), id);
        let resp = reqwest::Client::new()
            .delete(url)
            .header("token", code.as_str())
            .send()
            .await?;
        if resp.status() != StatusCode::OK {
            error!("code:{}, {}", code, resp.status().as_str());
        }
        Ok(())
    }

    pub async fn heartbeat(code: String, body:String) -> Result<bool>{
        let addr_prefix = CONFIG.read().await.server_addr();
        let url = format!("{}/api/user/heartbeat", &addr_prefix);
        let resp = reqwest::Client::new()
            .post(url)
            .header("token", &code)
            .body(body)
            .send()
            .await?;
        Ok(resp.status() == StatusCode::OK)
    }
}
