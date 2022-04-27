use crate::error::Result;
use crate::models::Area;
use log::{error, debug, info};
use reqwest::StatusCode;
use serde_json::{to_string, Value};
use std::env;
use std::path::PathBuf;
use tokio::fs::{create_dir_all, File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};

pub struct AreaService;

impl AreaService {
    pub async fn get_many_sub_areas(parent_area_ids: Vec<i64>) -> Vec<Area> {
        let mut ret = Vec::with_capacity(parent_area_ids.len());
        for id in parent_area_ids.into_iter() {
            ret.push(match Self::get_sub_areas(id).await {
                Ok(d) => d,
                Err(e) => {
                    error!("{:?}", e);
                    Area::new()
                }
            });
        }
        ret
    }

    pub async fn get_sub_areas(parent_area_id: i64) -> Result<Area> {
        for url in &[
            format!("https://fts.jd.com/area/get?fid={}", parent_area_id),
            format!("https://d.jd.com/area/get?fid={}", parent_area_id),
        ] {
            let resp = reqwest::get(url.as_str()).await?;
            let status = resp.status();
            if status == StatusCode::OK {
                let r: Value = resp.json().await?;
                let mut area = Area::new();
                if r.is_array() {
                    let x: &[_] = &['"'];
                    for v in r.as_array().unwrap() {
                        area.insert(
                            v["id"].as_i64().unwrap(),
                            v["name"].to_string().trim_matches(x).to_string(),
                        );
                    }
                } else {
                    error!(
                        "Get area error:{} - {} - {}",
                        url.as_str(),
                        status,
                        r.to_string()
                    );
                }
                return Ok(area);
            } else {
                error!("请求地区失败:{} - {}", url.as_str(), status);
            }
        }
        return Ok(Area::new());
    }

    fn area_file_path() -> PathBuf {
        let mut home = PathBuf::from(env::var(crate::HOME).unwrap());
        let file_path = home.join("db/area.txt");
        info!("{:?}", file_path);
        file_path
    }

    pub async fn load_area() -> Result<String> {
        let db_file_path = Self::area_file_path();
        match File::open(&db_file_path).await {
            Ok(db_file) => {
                let mut reader = BufReader::new(db_file);
                let mut ret = String::with_capacity(32);
                let _ = reader.read_to_string(&mut ret).await?;
                Ok(ret)
            }
            Err(e) => {
                use std::io::ErrorKind::NotFound;
                if e.kind() == NotFound {
                    let parent = db_file_path.parent().unwrap();
                    create_dir_all(parent).await?;
                    File::create(db_file_path.as_path()).await?;
                    return Ok("".to_string());
                }
                return Err(e.into());
            }
        }
    }

    pub async fn store_area(data: String) -> Result<()> {
        let db_file_path = Self::area_file_path();
        let mut db_file = OpenOptions::new()
            .write(true)
            .append(false)
            .truncate(true)
            .open(db_file_path)
            .await?;
        db_file.write_all(data.as_bytes()).await?;
        Ok(())
    }
}
