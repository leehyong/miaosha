mod app;
mod components;
mod flags;
mod message;
mod mywidget;
mod style;

pub use app::JdMiaoshaApp;
pub use components::get_headers;
use flags::AppFlags;
use iced::*;
pub use message::*;
use std::path::PathBuf;

pub const PORTION_1: u16 = 1;

pub fn run_app(exe_path: PathBuf) {
    let mut settings = Settings::with_flags(AppFlags::new(exe_path));
    let flags = settings.flags.exe_path.as_path().clone();
    settings.window.resizable = false; // 不能重新缩放窗口
    settings.window.size = (1200, 600);
    settings.default_font = Some(include_bytes!(
        "../../assets/font/ZiTiGuanJiaFangSongTi-2.ttf"
    ));
    JdMiaoshaApp::run(settings).unwrap();
}
