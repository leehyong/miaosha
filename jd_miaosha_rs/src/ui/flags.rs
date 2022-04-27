use std::path::PathBuf;

#[derive(Default, Debug, Clone)]
pub struct AppFlags{
    // 激活码
    pub activate_code: Option<String>,
    // 是否激活
    pub is_activate: bool,
    // 是否过期
    pub is_expire:bool,
    pub driver_inited:bool,
    // 程序的运行路径
    pub exe_path:PathBuf
}

impl AppFlags{
    pub fn new(exe_path:PathBuf) -> Self{
        let mut obj = Self::default();
        obj.exe_path = exe_path;
        obj
    }
}
