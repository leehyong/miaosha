use serde::{Serialize, Deserialize, Serializer, Deserializer};

#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub enum ProdPlatform {
    JD = 0
}

impl ProdPlatform {
    pub fn into_u8(self) -> u8 {
        match self {
            ProdPlatform::JD => 0,
        }
    }

    pub fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.serialize_u8(self.into_u8())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ProdPlatform, D::Error>
        where
            D: Deserializer<'de>,
    {
        let s = u8::deserialize(deserializer)?;
        Ok(s.into())
    }
}

impl From<u8> for ProdPlatform {
    fn from(level: u8) -> Self {
        match level {
            0 => ProdPlatform::JD,
            _ => unreachable!()
        }
    }
}

impl From<ProdPlatform> for u8 {
    fn from(pt: ProdPlatform) -> u8 {
        match pt {
            ProdPlatform::JD => 0,
        }
    }
}

impl Default for ProdPlatform {
    fn default() -> Self {
        ProdPlatform::JD
    }
}

impl std::fmt::Display for ProdPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProdPlatform::JD => {
                write!(f, "京东")
            }
        }
    }
}

#[derive(Copy, Debug, Clone, Deserialize)]
#[repr(u8)]
pub enum VipLevel {
    Normal = 0,
    // 以下就是 1 —— 9
    Vip1 = 1,
    Vip2 = 2,
    Vip3 = 3,
    Vip4 = 4,
    Vip5 = 5,
    Vip6 = 6,
    Vip7 = 7,
    Vip8 = 8,
    Vip9 = 9,
}


impl Default for VipLevel {
    fn default() -> Self {
        VipLevel::Normal
    }
}

impl VipLevel {
    pub fn into_u8(self) -> u8 {
        match self {
            VipLevel::Normal => 0,
            VipLevel::Vip1 => 1,
            VipLevel::Vip2 => 2,
            VipLevel::Vip3 => 3,
            VipLevel::Vip4 => 4,
            VipLevel::Vip5 => 5,
            VipLevel::Vip6 => 6,
            VipLevel::Vip7 => 7,
            VipLevel::Vip8 => 8,
            VipLevel::Vip9 => 9,
        }
    }

    pub fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.serialize_u8(self.into_u8())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<VipLevel, D::Error>
        where
            D: Deserializer<'de>,
    {
        let s = u8::deserialize(deserializer)?;
        Ok(s.into())
    }

    pub fn vip_level_seconds(self) -> i64 {
        // 每个vip级别能使用的天数
        const DAY_SECONDS: i64 = 24 * 3600;
        let level = self.into_u8();
        match level {
            1 | 2 | 3 => 60 * DAY_SECONDS,
            4 | 5 | 6 => 180 * DAY_SECONDS,
            7 | 8 | 9 => 365 * DAY_SECONDS,
            // 试用
            _ => 3 * DAY_SECONDS // 试用3天
        }
    }

    pub fn vip_level_max_accounts(self) -> usize{
        // 每个vip级别所支持的账户数量
        let level = self.into_u8();
        match level {
            // 初级vip
            1 => 3,
            2 => 5,
            3 => 8,
            // 中级vip
            4 => 10,
            5 => 15,
            6 => 20,
            // 高级vip
            7 => 30,
            8 => 40,
            9 => 50,
            // 试用
            _ => 1 // 试用版最多支持1个账号
        }
    }
}

impl From<u8> for VipLevel {
    fn from(level: u8) -> Self {
        match level {
            0 => VipLevel::Normal,
            1 => VipLevel::Vip1,
            2 => VipLevel::Vip2,
            3 => VipLevel::Vip3,
            4 => VipLevel::Vip4,
            5 => VipLevel::Vip5,
            6 => VipLevel::Vip6,
            7 => VipLevel::Vip7,
            8 => VipLevel::Vip8,
            9 => VipLevel::Vip9,
            _ => VipLevel::Normal
        }
    }
}

impl From<VipLevel> for u8 {
    fn from(level: VipLevel) -> u8 {
        level.into_u8()
    }
}




#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub enum Category {
    Computer = 0
}


impl Category {
    pub fn into_u8(self) -> u8 {
        match self {
            Category::Computer => 0,
        }
    }

    pub fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.serialize_u8(self.into_u8())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Category, D::Error>
        where
            D: Deserializer<'de>,
    {
        let s = u8::deserialize(deserializer)?;
        Ok(s.into())
    }
}

impl From<u8> for Category {
    fn from(level: u8) -> Self {
        match level {
            0 => Category::Computer,
            _ => unreachable!()
        }
    }
}

impl From<Category> for u8 {
    fn from(pt: Category) -> u8 {
        match pt {
            Category::Computer => 0,
        }
    }
}

impl Default for Category {
    fn default() -> Self {
        Category::Computer
    }
}