#[derive(PartialEq)]
#[derive(Debug)]
#[derive(Copy, Clone)]
pub enum Platform {
    Linux,
    Windows,
    MacOS,
    Unknown
}

impl Platform {
    pub fn to_string(self) -> &'static str {
        match self {
            Platform::Linux => "linux",
            Platform::Windows => "windows",
            Platform::MacOS => "macos",
            Platform::Unknown => "Unknown"
        }
    }

    pub fn current() -> Platform {
        if cfg!(unix) {
            Platform::MacOS
        }/*else if cfg!(unix) {
            Platform::Linux
        } */else if cfg!(windows) {
            Platform::Windows
        }else{
            Platform::Unknown
        }
    }
}