use std::cmp::Ordering;
use std::collections::{BTreeMap, LinkedList};
use std::sync::Arc;

use crate::*;
use crate::error::Result;
use crate::models::{AddressInfo, GoodsState, OrderInfo, UserInfo, UserState};
use crate::models::*;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum JdMiaoshaAppMessage {
    // 全局消息
    // 点击了商品按钮
    GlobalGoodsPressed,
    // 点击了购物车按钮
    GlobalShoppingCartPressed,
    // 点击了订单按钮
    GlobalOrdersPressed,
    // 点击了收货地址按钮
    GlobalDeliveryAddressPressed,
    // 点击了个人中心按钮
    GlobalPersonalCenterPressed,
    // 因为选择的地方比较多，所以整个全局选择的消息
    // 点击选择按钮
    GlobalClickSelection,
    // 选择中
    GlobalSelecting,
    // 已选好
    GlobalSelected,
    GlobalUnselect,
    GlobalNoop,

    InitDriverManager,
    InitDriverManagerFinish,
    InitDriverManagerFail,

    InitDriverClient,
    InitDriverClientFinish,
    InitDriverClientFail,

    // cookie 过期后， 全局都要重新更新
    GlobalCookieRefresh,
    // 商品消息
    Goods(GoodsMessage),
    // 地区消息
    Area(AreaMessage),

    //购物车消息
    ShoppingCart(ShoppingCartMessage),

    // 订单消息
    Order(OrderMessage),

    // 收货地址
    DeliveryAddress(DeliveryAddressMessage),
    // 个人中心消息
    PersonalCenterInputActivationCode,
    // 输入激活码
    PersonalCenterSubmitActivationCode, // 提交激活码

    // 订单消息
    User(UserMessage),

}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum GoodsMessage {
    // 针对整个商品表的消息
    // 商品输入
    SearchInput(String),
    // 商品查询
    Search,
    SearchFinish(Option<GoodsState>),
    ClearAll,

    QueryPurchaseLinkFinish(Option<(String, String)>),
    // 更新购买链接
    MaybeUpdatePurchaseLink(String, String),

    // 针对单个商品的消息
    // 把商品加入购物车
    AddToShoppingCart(String),
    AddToShoppingCartByPresale(String, Option<PKDateTime>, String),
    AddNum(String),
    SubNum(String),
    // 立即下单
    ImmediatelyPurchase(String),
    // 预约下单
    TimeoutPurchase(String),

    // 监控商品库存
    MonitorStock(IDType, String),
    Noop,
}

type Level = u8;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum AreaMessage {
    LevelChanged(Level, String),
    LevelChangedFinished(Level, Area),
    AreaStoreFinished,
    AreaLoad(String),
    AreaLoadFinished(Vec<Area>),
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum OrderMessage {
    Loading(String),
    LoadFinish(BTreeMap<String,OrderInfo>),
    LoadFailed,

    // 订单号输入
    SearchInput(String),
    Reset,
    // 订单查询
    Search,
    SearchFinish(BTreeMap<String, OrderInfo>),
    SearchFailed,

    // 订单物流信息
    ExpressInfo(String),
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum ShoppingCartMessage {
    // 购物车消息
    Loading,
    // fixme: 使用 Box， 避免大量复制
    LoadFinish(Option<ShoppingCartPageState>),
    LoadFail,
    PrevPage,
    NextPage,
    ClearAll,
    Select,
    DeleteSomeStates,
    ImmediatelySomeStates,
    SearchInput(String),
    Search,
    SearchReset,
    YuyueChecking(IDType),

    StockChecking,
    UpdateStock(IDType, StockStatus),

    Editing(IDType),
    EditInputTimeoutDt(IDType, String),
    EditFinish(IDType),
    EditCancel(IDType),
    ImmediatelyPurchase(IDType),
    DeleteOneState(IDType),
    SelectOne(IDType),
    SubNum(IDType),
    AddNum(IDType),
    // 加入购物车并提交订单
    SubmitOrder(IDType),
    MaybeRemoveShoppingCartLock(IDType),
    SubmitOrderFinish(IDType, &'static str),
    Noop,
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum DeliveryAddressMessage {
    // 设置收货地址
    Setting(String,String),
    SetFinish(String, String),
    SetFail,
    SearchInput(String),
    Search,
    SearchReset,
    LoadFinish(LinkedList<AddressInfo>),
    LoadFinishFailed,
}

macro_rules! msg_from {
    ($from:ty, $to:ty, $it:ident) => {
        impl From<$from> for $to {
            fn from(msg: $from) -> $to {
                <$to>::$it(msg)
            }
        }
    };
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum UserMessage {
    // 输入
    SearchInput(String),
    // 查询
    Search,
    SearchFinish(Vec<UserState>),
    SearchReset, // 重置搜索
    Noop, // 空操作
    // 针对单个商品的消息
    // 把商品加入购物车

    // 激活
    ActivateInput(String),
    Activate,
    ActivateFinish(Option<UserInfo>),
    // 登录某个账号
    UserOpMessage(EUserOpMessage),
    Select,
    // 删除全部账号
    ClearAll,
    // 删除选中的账号
    DeleteSomeStates,
    // 新增账号信息
    NewState,

    Loading,
    LoadFinish(AccountsPageState),
    LoadCodeFinish(Option<UserInfo>),
    Store,
    StoreFinish,
    // 导入账号信息
    Import,
    ImportFinish,

    AddToCartAndLoadCartGoodsSkuUuids(u8, Option<String>),

    CookieChecking,
    HeartBeat,
    HeartBeatFinish(bool),
    CookieCheckFinish(String, bool),
    CookieCheckFail,

    // 领取优惠券消息
    GetCoupon(String)
}

#[derive(Debug, Clone)]
pub struct UserCookieInfo{
    pub id:IDType,
    pub account:String,
    pub cookie:Arc<String>,
    pub eid:String,
    pub fp:String,
}

impl PartialEq for UserCookieInfo{
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for UserCookieInfo{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.id > other.id{
            Some(Ordering::Greater)
        }else if self.id < other.id{
            Some(Ordering::Less)
        }else{
            Some(Ordering::Equal)
        }
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum EUserOpMessage{
    NewStateFinish,
    NewStateCancel,
    EditStateFinish(IDType),
    EditStateCancel(IDType),

    NewInputAccount(String),
    NewInputPwd(String),

    EditInputAccount(String),
    EditInputPwd(String),
    ViewPwd(IDType),
    UpdateState(IDType),
    DeleteState(IDType),
    Selected(Option<IDType>),

    // 登录
    Login(IDType),
    LoginFinish(UserCookieInfo),
    LoginFail(String),
}

msg_from!(GoodsMessage, JdMiaoshaAppMessage, Goods);
msg_from!(AreaMessage, JdMiaoshaAppMessage, Area);
msg_from!(OrderMessage, JdMiaoshaAppMessage, Order);
msg_from!(ShoppingCartMessage, JdMiaoshaAppMessage, ShoppingCart);
msg_from!(DeliveryAddressMessage, JdMiaoshaAppMessage, DeliveryAddress);
msg_from!(UserMessage, JdMiaoshaAppMessage, User);
msg_from!(EUserOpMessage, UserMessage, UserOpMessage);
