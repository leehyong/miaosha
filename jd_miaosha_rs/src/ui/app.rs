use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::option::Option::Some;
use std::sync::{Arc, RwLock};

use iced::*;
use iced::rule::Style as RuleStyle;
use iced::widget::rule::FillMode;
use log::*;
use tokio::sync::Mutex;
use tokio::time::Duration;
use toml::to_string;

use crate::{IDType, PKLocal, PRESALE, SECOND_KILL, YUYUE};
use crate::models::{UserInfo, UserState};
use crate::services::area::AreaService;
use crate::services::delivery_address::DeliveryAddressService;
use crate::services::order::{OrderService, QueryCondition};
use crate::services::shopping_cart::ShoppingCartService;
use crate::services::user::UserService;
use crate::ui::components::DeliveryAddressComponent;
use crate::ui::components::GoodsComponent;
use crate::ui::components::OrderComponent;
use crate::ui::components::ShoppingCartComponent;
use crate::ui::components::UserComponent;
use crate::ui::UserMessage::Select;
use crate::utils::{greater_than_now, workers};

use super::{flags, message::*, style};
use super::components;
use super::PORTION_1;

#[derive(Default)]
pub struct JdMiaoshaApp {
    app_settings: flags::AppFlags,
    cur_tab: JdMiaoshaAppMessage,
    goods_button: button::State,
    shopping_cart_button: button::State,
    order_button: button::State,
    delivery_address_button: button::State,
    personal_center_button: button::State,
    // 商品表
    goods_table: Option<Box<GoodsComponent>>,
    user_table: Option<Box<UserComponent>>,
    address_table: Option<Box<DeliveryAddressComponent>>,
    shopping_cart_table: Option<Box<ShoppingCartComponent>>,
    order_table: Option<Box<OrderComponent>>,
}

lazy_static! {
    static ref ORDER_CHECK_TIMES: Arc<RwLock<HashMap<IDType, usize>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

impl Default for JdMiaoshaAppMessage {
    fn default() -> Self {
        Self::GlobalGoodsPressed
    }
}

impl JdMiaoshaApp {
    pub fn set_check_times(id: IDType, times: usize) -> bool {
        match ORDER_CHECK_TIMES.write() {
            Ok(mut guard) => {
                guard.insert(id, times);
                true
            }
            Err(e) => {
                error!("{:?}", e);
                false
            }
        }
    }
    pub fn get_check_times(id: IDType) -> usize {
        match ORDER_CHECK_TIMES.read() {
            Ok(guard) => guard.get(&id).unwrap_or(&0).to_owned(),
            Err(e) => {
                error!("{:?}", e);
                0
            }
        }
    }

    pub fn button_style(stat1: JdMiaoshaAppMessage, stat2: JdMiaoshaAppMessage) -> style::Button {
        if stat1 == stat2 {
            style::Button::Primary
        } else {
            style::Button::Secondary
        }
    }

    fn is_activate(&self) -> bool {
        // 是否激活
        if let Some(ref u) = self.user_table {
            return u.is_activate;
        }
        false
    }

    fn is_activate_and_have_users(&self) -> bool {
        self.is_activate()
            && self.user_table.is_some()
            && self
                .user_table
                .as_ref()
                .unwrap()
                .user_info
                .users
                .records
                .len()
                > 0
    }

    fn init_driver() -> Command<JdMiaoshaAppMessage> {
        Command::perform(UserService::init_web_driver(), |r| match r {
            Ok(_) => JdMiaoshaAppMessage::InitDriverManagerFinish,
            Err(e) => {
                error!("{:?}", e);
                JdMiaoshaAppMessage::InitDriverManagerFail
            }
        })
    }

    fn submit_order(
        account_id: IDType,
        cookie: Arc<String>,
        card_goods_id: IDType,
        sku: String,
        num: u32,
        area: String,
        _workers: usize,
    ) -> Vec<Command<JdMiaoshaAppMessage>> {
        return vec![Command::perform(
            ShoppingCartService::submit_order_wrapper(
                account_id,
                cookie.clone(),
                card_goods_id,
                sku.clone(),
                num,
                area.clone(),
                false,
                _workers,
            ),
            |r| match r {
                Ok((id, num, status)) => {
                    ShoppingCartMessage::SubmitOrderFinish(id, status).into()
                }
                Err(e) => {
                    error!("提交订单失败！{:?}", e);
                    ShoppingCartMessage::Noop.into()
                }
            },
        )];
    }

    fn submit_yuyue_order(
        account_id: IDType,
        cookie: Arc<String>,
        card_goods_id: IDType,
        sku: String,
        num: u32,
        in_yuyue: bool,
        area: String,
    ) -> Command<JdMiaoshaAppMessage> {
        Command::perform(
            async move {
                ShoppingCartService::submit_yuyue_order(
                    account_id,
                    cookie,
                    card_goods_id,
                    sku,
                    num,
                    in_yuyue,
                    area,
                )
                    .await
            },
            |r| match r {
                Ok((id, num, status)) => ShoppingCartMessage::SubmitOrderFinish(id, status).into(),
                Err(e) => {
                    error!("提交预约订单失败！{:?}", e);
                    ShoppingCartMessage::Noop.into()
                }
            },
        )
    }

    fn submit_presale_order(
        account_id: IDType,
        cookie: Arc<String>,
        card_goods_id: IDType,
        sku: String,
        eid: String,
        fp: String,
        num: u32,
    ) -> Command<JdMiaoshaAppMessage> {
        Command::perform(
            async move {
                ShoppingCartService::submit_presale_order(
                    account_id,
                    cookie,
                    card_goods_id,
                    sku,
                    eid,
                    fp,
                    num,
                )
                .await
            },
            |r| match r {
                Ok((id, num, status)) => ShoppingCartMessage::SubmitOrderFinish(id, status).into(),
                Err(e) => {
                    error!("提交预约订单失败！{:?}", e);
                    ShoppingCartMessage::Noop.into()
                }
            },
        )
    }

    fn submit_seckill_order(
        account_id: IDType,
        cookie: Arc<String>,
        card_goods_id: IDType,
        sku: String,
        eid: String,
        fp: String,
    ) -> Command<JdMiaoshaAppMessage> {
        Command::perform(
            async move {
                ShoppingCartService::submit_seckill_order(
                    account_id,
                    cookie,
                    card_goods_id,
                    sku,
                    eid,
                    fp,
                )
                .await
            },
            |r| match r {
                Ok((id, num, status)) => {
                    return ShoppingCartMessage::SubmitOrderFinish(id, status).into();
                }
                Err(e) => {
                    error!("提交秒杀抢购订单失败！{:?}", e);
                    ShoppingCartMessage::Noop.into()
                }
            },
        )
    }
}

impl Application for JdMiaoshaApp {
    // type Executor = executor::Default;
    type Executor = crate::executor::MiaoshaExecutor;
    type Message = JdMiaoshaAppMessage;
    type Flags = flags::AppFlags;

    fn new(flags: flags::AppFlags) -> (Self, Command<Self::Message>) {
        let mut obj = Self::default();
        obj.cur_tab = JdMiaoshaAppMessage::GlobalGoodsPressed;
        obj.goods_table = Some(Box::new(GoodsComponent::default()));
        obj.user_table = Some(Box::new(UserComponent::default()));
        obj.address_table = Some(Box::new(DeliveryAddressComponent::default()));
        obj.order_table = Some(Box::new(OrderComponent::default()));
        obj.shopping_cart_table = Some(Box::new(ShoppingCartComponent::new()));
        obj.app_settings = flags;
        (obj, Self::init_driver())
    }

    fn title(&self) -> String {
        String::from("抢购秒杀")
    }

    fn update(
        &mut self,
        message: Self::Message,
        clipboard: &mut Clipboard,
    ) -> Command<Self::Message> {
        use JdMiaoshaAppMessage::*;
        // info!("{:?}", &message);
        match message {
            GlobalGoodsPressed => {
                self.cur_tab = GlobalGoodsPressed;
            }
            GlobalShoppingCartPressed => {
                self.cur_tab = GlobalShoppingCartPressed;
                if self.is_activate() {
                    return Command::perform(async move {}, |_| {
                        ShoppingCartMessage::Loading.into()
                    });
                }
            }
            GlobalOrdersPressed => {
                self.cur_tab = GlobalOrdersPressed;
                if self.is_activate_and_have_users() {
                    return Command::batch(
                        self.user_table
                            .as_ref()
                            .unwrap()
                            .user_info
                            .users
                            .records
                            .iter()
                            .map(|u| {
                                let account = u.account.clone();
                                let cookie = u.cookie.clone();
                                Command::perform(
                                    async {
                                        OrderService::get_orders_by_user(
                                            account,
                                            cookie,
                                            QueryCondition::Unpaid,
                                        )
                                        .await
                                    },
                                    |r| match r {
                                        Ok(info) => OrderMessage::LoadFinish(info).into(),
                                        Err(e) => {
                                            error!("Load address error: {:?}", e);
                                            OrderMessage::LoadFailed.into()
                                        }
                                    },
                                )
                            }),
                    );
                }
            }
            GlobalDeliveryAddressPressed => {
                self.cur_tab = GlobalDeliveryAddressPressed;
                if self.is_activate_and_have_users() {
                    return Command::batch(
                        self.user_table
                            .as_ref()
                            .unwrap()
                            .user_info
                            .users
                            .records
                            .iter()
                            .map(|u| {
                                let account = u.account.clone();
                                let cookie = u.cookie.clone();
                                Command::perform(
                                    async {
                                        DeliveryAddressService::get_all_address_by_user(
                                            account, cookie,
                                        )
                                        .await
                                    },
                                    |r| match r {
                                        Ok(info) => DeliveryAddressMessage::LoadFinish(info).into(),
                                        Err(e) => {
                                            error!("Load address error: {:?}", e);
                                            DeliveryAddressMessage::LoadFinishFailed.into()
                                        }
                                    },
                                )
                            }),
                    );
                }
            }

            GlobalPersonalCenterPressed => {
                self.cur_tab = GlobalPersonalCenterPressed;
            }
            Goods(GoodsMessage::MaybeUpdatePurchaseLink(sku, url)) => {
                if self.is_activate() {
                    let code = self
                        .user_table
                        .as_ref()
                        .unwrap()
                        .user_info
                        .activate_code
                        .to_owned();
                    let body = serde_json::json!({
                        "sku":sku,
                        "purchase_url":url
                    })
                    .to_string();
                    return Command::perform(
                        async move {
                            ShoppingCartService::update_cart_goods_purchase_link(code, body).await
                        },
                        |_| ShoppingCartMessage::Loading.into(),
                    );
                }
            }
            Goods(gs) => {
                return self.goods_table.as_mut().unwrap().update(gs);
            }
            InitDriverManager => {
                return Self::init_driver();
            }
            InitDriverManagerFinish => {
                self.app_settings.driver_inited = true;
                return Command::batch(vec![
                    Command::perform(async {}, |_| UserMessage::Loading.into()),
                    Command::perform(async {}, |_| InitDriverClient),
                    Command::perform(async { AreaService::load_area().await }, |t| {
                        if t.is_err() {
                            error!("{:?}", t);
                        }
                        AreaMessage::AreaLoad(t.unwrap_or_default()).into()
                    }),
                ]);
            }
            InitDriverClient => {
                return Command::perform(async { UserService::init_driver_clients().await }, |r| {
                    match r {
                        Ok(_) => InitDriverClientFinish,
                        Err(_) => InitDriverClientFail,
                    }
                });
            }
            InitDriverManagerFail => {
                error!("InitDriverManagerFail");
            }

            Area(as_) => {
                self.cur_tab = GlobalGoodsPressed;
                return self.goods_table.as_mut().unwrap().update_area(as_);
            }
            User(UserMessage::HeartBeat) => {
                if let Some(table) = &self.user_table {
                    let mac_addr = UserInfo::get_user_mac_address();
                    let code = table.user_info.activate_code.clone();
                    let body = serde_json::json!({ "mac": mac_addr }).to_string();
                    return Command::perform(
                        async move { UserService::heartbeat(code, body).await },
                        |r| {
                            match r {
                                Ok(b) => User(UserMessage::HeartBeatFinish(b)).into(),
                                Err(e) => {
                                    error!("心跳检测失败{:?}", e);
                                    // 检测失败时，直接视为心跳失败，那就回退出本程序
                                    User(UserMessage::HeartBeatFinish(false)).into()
                                }
                            }
                        },
                    );
                }
            }
            User(UserMessage::HeartBeatFinish(b)) => {
                if let Some(table) = &mut self.user_table {
                    if b {
                        // 心跳检测成功就把这个计数器置 0
                        table.heartbeat_error_times = 0;
                    } else {
                        table.heartbeat_error_times += 1;
                    }
                    info!("HeartBeatFinish:{}", b);
                    if table.heartbeat_error_times > 1 {
                        info!("激活码已被使用过, 请获取新的激活码");
                        // 连续两次检测失败就把激活置为false， 然后整个程序，涉及到激活的操作就不能再使用了
                        table.is_activate = false;
                        if let Some(table) = &mut self.goods_table {
                            table.activate_code.clear();
                        }
                        if let Some(table) = &mut self.shopping_cart_table {
                            table.activate_code.clear();
                        }
                    }
                }
            }
            User(umsg) => {
                if self.is_activate() {
                    let code = &self.user_table.as_ref().unwrap().user_info.activate_code;
                    if let Some(shopping_cart) = &mut self.shopping_cart_table {
                        shopping_cart.activate_code = code.clone();
                    }
                    if let Some(goods_table) = &mut self.goods_table {
                        goods_table.activate_code = code.clone();
                    }
                }
                return self
                    .user_table
                    .as_mut()
                    .unwrap()
                    .update(umsg, self.app_settings.driver_inited.clone());
            }
            DeliveryAddress(dmsg) => {
                return self.address_table.as_mut().unwrap().update(dmsg);
            }

            ShoppingCart(ShoppingCartMessage::MaybeRemoveShoppingCartLock(id)) => {
                return Command::none();
            }

            ShoppingCart(ShoppingCartMessage::SubmitOrder(id)) => {
                if !self.is_activate() {
                    // 没激活， 跳过
                    return Command::none();
                }
                if let Some(cart_item) = self
                    .shopping_cart_table
                    .as_mut()
                    .unwrap()
                    .prods
                    .records
                    .iter_mut()
                    .find(|item| item.id == id && item.purchase_status != "purchasing")
                {
                    info!("SubmitOrder buying: {}-{}", id, &cart_item.sku);
                    let sku = cart_item.sku.clone();
                    let num = cart_item.purchase_num;
                    let area = self.goods_table.as_ref().unwrap().get_addr_str();
                    let purchase_url = cart_item.purchase_url.clone();
                    let mut valid_users = vec![];
                    let cmds: Vec<Command<JdMiaoshaAppMessage>> = self
                        .user_table
                        .as_ref()
                        .unwrap()
                        .user_info
                        .users
                        .records
                        .iter()
                        .map(|item| {
                            valid_users.push(item.account.clone());
                            let account_id = item.id;
                            let cookie = item.cookie.clone();
                            let _sku = sku.clone();
                            let num = num.clone();
                            if cart_item.purchase_type.eq(SECOND_KILL) {
                                (1..=workers())
                                    .into_iter()
                                    .map(|_| {
                                        Self::submit_seckill_order(
                                            account_id,
                                            cookie.clone(),
                                            id,
                                            sku.clone(),
                                            item.eid.clone(),
                                            item.fp.clone(),
                                        )
                                    })
                                    .collect::<Vec<_>>()
                            } else if cart_item.purchase_type.eq(PRESALE) {
                                if let Some(ref yuyue_dt) = cart_item.yuyue_dt {
                                    if greater_than_now(yuyue_dt) {
                                        // 还不到设定的预约时间， 不能购买
                                        warn!(
                                            "预售商品还不到设定的预约时间,不能购买: {}",
                                            serde_json::to_string(&cart_item).unwrap_or_default()
                                        );
                                        return vec![];
                                    }
                                }
                                (1..=workers())
                                    .into_iter()
                                    .map(|_| {
                                        Self::submit_presale_order(
                                            account_id,
                                            cookie.clone(),
                                            id,
                                            sku.clone(),
                                            item.eid.clone(),
                                            item.fp.clone(),
                                            num,
                                        )
                                    })
                                    .collect::<Vec<_>>()
                            } else if cart_item.purchase_type.eq(YUYUE) {
                                if cart_item.purchase_status.eq("ready") {
                                    let is_yuyue = if let Some(ref end_dt) = cart_item.yuyue_end_dt
                                    {
                                        greater_than_now(end_dt)
                                    } else {
                                        false
                                    };
                                    if is_yuyue {
                                        info!("{}-{}:预约商品...", account_id, sku);
                                    } else {
                                        info!(
                                            "{}-{}:跳过预约，直接购买预约商品...",
                                            account_id, sku
                                        );
                                    }
                                    vec![Self::submit_yuyue_order(
                                        account_id,
                                        cookie.clone(),
                                        id,
                                        sku.clone(),
                                        num,
                                        is_yuyue,
                                        area.clone(),
                                    )]
                                } else if cart_item.purchase_status.eq("yuyueing")
                                    && !greater_than_now(cart_item.yuyue_dt.as_ref().unwrap())
                                {
                                    // 预约时间到了就去真正购买
                                    info!("{}-{}:购买已经预约过的商品", account_id, sku);
                                    // 把并发预约放到 submit_yuyue_order 里， 因为可能导致预约的商品在购物车的数量不对
                                    vec![Self::submit_yuyue_order(
                                        account_id,
                                        cookie.clone(),
                                        id,
                                        sku.clone(),
                                        num,
                                        false,
                                        area.clone(),
                                    )]
                                } else {
                                    warn!(
                                        "{}-{}:预约商品的状态不对,不能购买: {}!",
                                        account_id, sku, cart_item.purchase_status
                                    );
                                    // warn!("Ignored yuyue: {}", serde_json::to_string(&cart_item).unwrap_or_default());
                                    vec![]
                                }
                            } else {
                                let mut _workers = 1;
                                if let Some(ref yuyue_dt) = cart_item.yuyue_dt {
                                    if greater_than_now(yuyue_dt) {
                                        // 还不到设定的预约时间， 不能购买
                                        warn!(
                                            "普通商品预约时间不到,不能购买: {}",
                                            serde_json::to_string(&cart_item).unwrap_or_default()
                                        );
                                        return vec![];
                                    }
                                    _workers = workers();
                                }
                                Self::submit_order(
                                    account_id,
                                    cookie.clone(),
                                    id,
                                    _sku,
                                    num,
                                    area.clone(),
                                    _workers,
                                )
                            }
                        })
                        .flatten()
                        .collect();
                    Self::set_check_times(cart_item.id, cmds.len());
                    if cmds.len() > 0 {
                        cart_item.purchase_status = "purchasing".to_string();
                    }
                    let m = std::cmp::min(5, valid_users.len());
                    info!(
                        "购买商品:{}-{},参与用户:{}{}{}人,总次数: {}, [May be concurrent workers:{}]!",
                        cart_item.id,
                        sku,
                        valid_users.join(","),
                        if valid_users.len() > m{
                            "...等"
                        }else{
                            ",共"
                        },
                        valid_users.len(),
                        cmds.len(),
                        workers()
                    );
                    return Command::batch(cmds);
                }
            }

            ShoppingCart(msg) => {
                return self.shopping_cart_table.as_mut().unwrap().update(msg);
            }

            Order(OrderMessage::Search) => {
                if self.is_activate_and_have_users() {
                    let cond = QueryCondition::KeyWord(
                        self.order_table.as_ref().unwrap().search_input_txt.clone(),
                    );
                    self.order_table.as_mut().unwrap().search_input_txt.clear();
                    return Command::batch(
                        self.user_table
                            .as_mut()
                            .unwrap()
                            .user_info
                            .users
                            .records
                            .iter_mut()
                            .filter(|u| {
                                UserState::maybe_valid_cookie(u.cookie_last_update_dt.clone())
                            })
                            .map(move |u| {
                                let account = u.account.clone();
                                let cookie = u.cookie.clone();
                                let cond = cond.clone();
                                Command::perform(
                                    async move {
                                        OrderService::get_orders_by_user(account, cookie, cond)
                                            .await
                                    },
                                    |r| match r {
                                        Ok(orders) => OrderMessage::SearchFinish(orders).into(),
                                        Err(e) => {
                                            error!("{:?}", e);
                                            OrderMessage::SearchFailed.into()
                                        }
                                    },
                                )
                            }),
                    );
                }
            }

            Order(msg) => {
                return self.order_table.as_mut().unwrap().update(msg);
            }

            _ => {
                return Command::none();
            }
        }
        return Command::none();
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let mut subscriptions = vec![];
        if !self.app_settings.driver_inited {
            info!("driver_inited restart");
            subscriptions.push(
                time::every(Duration::from_millis(3000))
                    .map(|_| JdMiaoshaAppMessage::InitDriverManager),
            );
        }
        let check_cookie = self.is_activate_and_have_users();
        if check_cookie {
            // 每隔一段时间检查 cookie, 使cookie保活
            subscriptions.push(
                time::every(Duration::from_secs(180)).map(|_| UserMessage::CookieChecking.into()),
            );
            // 定时检测商品库存
            subscriptions.push(
                time::every(Duration::from_secs(3))
                    .map(|_| ShoppingCartMessage::StockChecking.into()),
            );
        }
        //每隔一段时间无条件检查激活条件
        subscriptions
            .push(time::every(Duration::from_secs(300)).map(|_| UserMessage::HeartBeat.into()));
        Subscription::batch(subscriptions)
    }

    fn view(&mut self) -> Element<Self::Message> {
        let mut first_row_children = Vec::with_capacity(5);
        for child in vec![
            (
                &mut self.goods_button,
                "商品",
                JdMiaoshaAppMessage::GlobalGoodsPressed,
            ),
            (
                &mut self.shopping_cart_button,
                "购物车",
                JdMiaoshaAppMessage::GlobalShoppingCartPressed,
            ),
            (
                &mut self.order_button,
                "订单",
                JdMiaoshaAppMessage::GlobalOrdersPressed,
            ),
            (
                &mut self.delivery_address_button,
                "收货地址",
                JdMiaoshaAppMessage::GlobalDeliveryAddressPressed,
            ),
            (
                &mut self.personal_center_button,
                "个人中心",
                JdMiaoshaAppMessage::GlobalPersonalCenterPressed,
            ),
        ]
        .into_iter()
        {
            first_row_children.push(
                Button::new(
                    child.0,
                    Text::new(child.1)
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .size(30),
                )
                .width(Length::FillPortion(PORTION_1))
                .on_press(child.2.clone())
                .style(JdMiaoshaApp::button_style(
                    self.cur_tab.clone(),
                    child.2.clone(),
                ))
                .into(),
            );
        }
        let first_row = Row::with_children(first_row_children)
            .padding(0)
            .align_items(Align::Center);
        let second_row = Row::new().push(
            Container::new(Row::new())
                .width(Length::Fill)
                .height(Length::Units(10))
                .style(style::AppBackgroundColor),
        );
        let content_box = match self.cur_tab {
            JdMiaoshaAppMessage::GlobalGoodsPressed => self.goods_table.as_mut().unwrap().view(),
            JdMiaoshaAppMessage::GlobalPersonalCenterPressed => {
                self.user_table.as_mut().unwrap().view()
            }
            JdMiaoshaAppMessage::GlobalShoppingCartPressed => {
                self.shopping_cart_table.as_mut().unwrap().view()
            }
            JdMiaoshaAppMessage::GlobalDeliveryAddressPressed => {
                self.address_table.as_mut().unwrap().view()
            }
            JdMiaoshaAppMessage::GlobalOrdersPressed => self.order_table.as_mut().unwrap().view(),
            _ => unreachable!(),
        };
        let layout = Column::new()
            .push(first_row)
            .push(second_row)
            .push(content_box);
        Container::new(layout).padding(4).into()
    }
}
