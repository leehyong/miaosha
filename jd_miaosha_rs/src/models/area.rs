use std::collections::BTreeMap;

pub type Area = BTreeMap<i64, String>;
pub static DEFAULT_ADDR:&'static str = "1_72_2839_0"; // 北京(1), 朝阳区(72), 四环到五环之间(2839);
pub static DEFAULT_ADDR_NAMES:&'static str = "北京_朝阳区_四环到五环之间_"; // 北京(1), 朝阳区(72), 四环到五环之间(2839);

lazy_static!{
    pub static ref PROVINCES:Area = {
      let mut ps = Area::new();
        ps.insert(1, "北京".to_string());
        ps.insert(2, "上海".to_string());
        ps.insert(3, "天津".to_string());
        ps.insert(4, "重庆".to_string());
        ps.insert(5, "河北".to_string());
        ps.insert(6, "山西".to_string());
        ps.insert(7, "河南".to_string());
        ps.insert(8, "辽宁".to_string());
        ps.insert(9, "吉林".to_string());
        ps.insert(10, "黑龙江".to_string());
        ps.insert(11, "内蒙古".to_string());
        ps.insert(12, "江苏".to_string());
        ps.insert(13, "山东".to_string());
        ps.insert(14, "安徽".to_string());
        ps.insert(15, "浙江".to_string());
        ps.insert(16, "福建".to_string());
        ps.insert(17, "湖北".to_string());
        ps.insert(18, "湖南".to_string());
        ps.insert(19, "广东".to_string());
        ps.insert(20, "广西".to_string());
        ps.insert(21, "江西".to_string());
        ps.insert(22, "四川".to_string());
        ps.insert(23, "海南".to_string());
        ps.insert(24, "贵州".to_string());
        ps.insert(25, "云南".to_string());
        ps.insert(26, "西藏".to_string());
        ps.insert(27, "陕西".to_string());
        ps.insert(28, "甘肃".to_string());
        ps.insert(29, "青海".to_string());
        ps.insert(30, "宁夏".to_string());
        ps.insert(31, "新疆".to_string());
        ps.insert(32, "台湾".to_string());
        ps.insert(52993, "港澳".to_string());
        ps.insert(84, "钓鱼岛".to_string());
    ps
    };
    pub static ref PROVINCE_NAMES:Vec<String> = PROVINCES.values().map(|s|s.to_owned()).collect::<Vec<String>>();
}