use std::collections::{HashMap, HashSet};
use std::collections::LinkedList;
use std::ops::{Deref, Sub};
use std::sync::{Arc, RwLock};

use iced::*;
use iced::pane_grid::Line;
use log::{debug, error, info, set_max_level, warn};
use serde_json::{json, to_string};
use thirtyfour::common::command::By::Id;

use crate::{IDType, PKDate, PKDateTime, PKLocal};
use crate::models::{UserInfo, UserInfoStatus, UserState};
use crate::services::shopping_cart::ShoppingCartService;
use crate::services::user::UserService;
use crate::utils::*;

use super::PORTION_1;
use super::style;
use super::super::*;

lazy_static! {
    static ref GET_USER_COUPONS: Arc<RwLock<HashMap<IDType, HashSet<String>>>> = Default::default();
}

#[derive(Default)]
pub struct UserComponent {
    // 商品搜索输入框的内容
    // 滚动条
    is_selecting: bool,
    pub is_activate: bool,
    pub heartbeat_error_times: u8,
    scroll_state: scrollable::State,
    // 商品搜索输入框
    // 商品搜索按钮
    new_name_state: text_input::State,
    new_name_txt: String,
    new_pwd_state: text_input::State,
    new_pwd_txt: String,
    // 搜索
    search_input_txt: String,
    search_input_state: text_input::State,
    search_button_state: button::State,
    search_reset_button_state: button::State,
    // 激活
    activate_input_state: text_input::State,
    activate_button_state: button::State,

    select_button_state: button::State,
    cookie_check_button_state: button::State,
    new_button_state: button::State,
    import_button_state: button::State,
    delete_button_state: button::State,
    clear_button_state: button::State,
    // 用户信息
    pub user_info: UserInfo,
}

impl UserComponent {
    pub const ACCOUNT_PORTION: u16 = 2;
    pub const APP_CK_PORTION: u16 = 3;
    pub const PWD_PORTION: u16 = 2;
    pub const OP_PORTION: u16 = 3;
    pub const RIGHT_PORTION: u16 = 4;

    fn left_right_portions(_: bool) -> (Vec<String>, Vec<u16>) {
        let mut headers = Vec::with_capacity(5);
        let mut portions = Vec::with_capacity(5);
        let _portions = [
            Self::ACCOUNT_PORTION,
            Self::PWD_PORTION,
            Self::APP_CK_PORTION,
            Self::OP_PORTION,
        ];
        for t in ["账号", "密码", "APP_CK", "操作"].iter() {
            headers.push(t.to_string())
        }
        portions.extend(&_portions);
        (headers, portions)
    }

    fn have_newed(&self) -> bool {
        if let Some(u) = self.user_info.users.records.get(0) {
            return u.status == UserInfoStatus::NewIng;
        }
        return false;
    }

    fn update_user_info(
        &mut self,
        code: String,
        msg: &EUserOpMessage,
    ) -> Command<JdMiaoshaAppMessage> {
        Command::batch(
            self.user_info
                .users
                .records
                .iter_mut()
                .map(|s| s.update(code.clone(), msg).map(|m| m.into())),
        )
    }

    pub fn update(
        &mut self,
        message: UserMessage,
        is_inited_web_driver: bool,
    ) -> Command<JdMiaoshaAppMessage> {
        use UserMessage::*;
        match &message {
            Loading | LoadCodeFinish(_) | LoadFinish(_) | Activate | ActivateInput(_)
            | ActivateFinish(_) => {}
            _ => {
                if !self.is_activate {
                    // 没有激活， 则不允许后续操作
                    return Command::none();
                }
            }
        }
        // 记录是否点击了选择按钮，避免发送过多的消息
        let mut is_select_change = false;
        let refresh_data = |t: crate::error::Result<_>| {
            if t.is_ok() {
                SearchReset.into()
            } else {
                Noop.into()
            }
        };
        match message {
            Loading => {
                return Command::perform(
                    async { UserService::load_user_activate_info().await },
                    |res| match res {
                        Ok(t) => {
                            return LoadCodeFinish(t).into();
                        }
                        Err(e) => {
                            error!("{:?}", e);
                            return LoadCodeFinish(None).into();
                        }
                    },
                );
            }
            LoadFinish(d) => {
                self.user_info.users = d;
                if self.user_info.users.records.len() > 0 {
                    // 加载出了用户后，需要执行 cookie 验证
                    return Command::perform(async {}, |_| UserMessage::CookieChecking.into());
                }
            }
            LoadCodeFinish(d) => {
                if let Some(uinfo) = d {
                    self.user_info = uinfo;
                    self.is_activate = !self.user_info.is_expired();
                    let code = self.user_info.activate_code.clone();
                    // 激活码加载完之后加载用户账号列表
                    return Command::perform(
                        async { UserService::list_accounts(code, "".to_string(), 1).await },
                        |res| {
                            match res {
                                Ok(t) => {
                                    if let Some(rdata) = t {
                                        return LoadFinish(rdata).into();
                                    }
                                }
                                Err(e) => {
                                    error!("{:?}", e);
                                }
                            }
                            return LoadFinish(Default::default()).into();
                        },
                    );
                }
            }

            Store => match serde_json::to_string(&self.user_info) {
                Ok(user_info) => {
                    return Command::perform(
                        async move { UserService::store_user_activate_info(user_info).await },
                        |t| {
                            if let Err(e) = t {
                                error!("{:?}", e);
                            }
                            StoreFinish.into()
                        },
                    );
                }
                Err(e) => {
                    error!("{:?}", e);
                    return Command::none();
                }
            },

            StoreFinish => {
                info!("用户数据存储完成-StoreFinish");
            }

            ActivateInput(code) => {
                self.user_info.activate_code = code.trim().to_string();
            }

            AddToCartAndLoadCartGoodsSkuUuids(num, sku) => {
                return Command::perform(
                    ShoppingCartService::add_to_cat_and_get_sku_uuid_from_account_cart(
                        num as u32,
                        sku,
                        self.user_info
                            .users
                            .records
                            .iter()
                            .filter(|u| u.maybe_valid_cookie2())
                            .map(|u| (u.id.clone(), u.cookie.clone()))
                            .collect(),
                    ),
                    |_| JdMiaoshaAppMessage::GlobalNoop,
                );
            }
            GetCoupon(sku) => {
                return Command::batch(
                    self.user_info
                        .users
                        .records
                        .iter()
                        .filter(|u| u.maybe_valid_cookie2())
                        .map(|u| match GET_USER_COUPONS.read() {
                            Ok(guard) => {
                                if let Some(set) = guard.get(&u.id) {
                                    if set.contains(&sku) {
                                        return Command::none();
                                    }
                                }
                                let sku = sku.clone();
                                let ck = u.cookie.clone();
                                let account_id = u.id;
                                return Command::perform(
                                    async move {
                                        ShoppingCartService::get_and_receive_coupons(
                                            ck, sku, account_id,
                                        )
                                            .await
                                    },
                                    |r| {
                                        match r {
                                            Ok((account_id, sku)) => {
                                                match GET_USER_COUPONS.write() {
                                                    Ok(mut guard) => {
                                                        let set =
                                                            guard.entry(account_id).or_default();
                                                        set.insert(sku);
                                                    }
                                                    Err(e) => {
                                                        error!(
                                                            "获取GET_USER_COUPONS写锁失败, 用户:{}",
                                                            account_id
                                                        )
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                error!("领取优惠券失败{:?}", e);
                                            }
                                        }
                                        JdMiaoshaAppMessage::GlobalNoop
                                    },
                                );
                            }
                            Err(e) => {
                                warn!("获取GET_USER_COUPONS读锁失败, 用户:{}, {:?}", u.id, e);
                                Command::none()
                            }
                        }),
                );
            }

            Activate => {
                if self.user_info.activate_code.len() > 0 {
                    let code = self.user_info.activate_code.clone();
                    return Command::perform(
                        async move { UserService::activate(code).await },
                        |t| {
                            if let Ok(u) = t {
                                return ActivateFinish(u).into();
                            }
                            error!("{:?}", t);
                            return ActivateFinish(None).into();
                        },
                    );
                }
            }

            ActivateFinish(u) => {
                if let Some(u) = u {
                    self.user_info = u;
                    self.is_activate = !self.user_info.is_expired();
                    // 激活完成后，把用户激活信息存到json文件，并且获取用户的抢购账号信息
                    return Command::batch(vec![
                        Command::perform(async {}, |_| UserMessage::Store.into()),
                        Command::perform(async {}, |_| UserMessage::SearchReset.into()),
                    ]);
                } else {
                    self.is_activate = false;
                }
            }

            SearchInput(name) => {
                self.search_input_txt = name.trim().to_string();
            }
            Search => {
                let kw = self.search_input_txt.clone();
                self.search_input_txt.clear();
                if self.user_info.activate_code.len() > 0 {
                    let code = self.user_info.activate_code.clone();
                    return Command::perform(
                        async { UserService::list_accounts(code, kw, 1).await },
                        |res| {
                            match res {
                                Ok(t) => {
                                    if let Some(rdata) = t {
                                        return LoadFinish(rdata).into();
                                    }
                                }
                                Err(e) => {
                                    error!("{:?}", e);
                                }
                            }
                            return LoadFinish(Default::default()).into();
                        },
                    );
                }
            }

            SearchReset => {
                self.search_input_txt.clear();
                self.is_selecting = false;
                is_select_change = true;
                // 重置搜索后， 需要从服务器上重新拉取数据
                return Command::perform(async {}, |_| Search.into());
            }
            ClearAll => {
                self.search_input_txt.clear();
                self.is_selecting = false;
                is_select_change = true;
                let code = self.user_info.activate_code.clone();
                self.user_info.users.records.clear();
                return Command::perform(
                    async move { UserService::delete_accounts(code, None).await },
                    |_| UserMessage::Noop.into(),
                );
            }

            UserOpMessage(EUserOpMessage::DeleteState(s)) => {
                let code = self.user_info.activate_code.clone();
                return Command::perform(
                    async move { UserService::delete_account(code, s).await },
                    refresh_data,
                );
            }

            NewState => {
                // 找头部是否已有新增的元素。 无，则新增；有，则略过。以避免无限增加元素
                if !self.have_newed() {
                    self.user_info.users.records.insert(
                        0,
                        UserState {
                            status: UserInfoStatus::NewIng,
                            ..Default::default()
                        },
                    );
                }
            }
            UserOpMessage(EUserOpMessage::NewStateFinish) => {
                if self.have_newed() {
                    let newed = &mut self.user_info.users.records[0];
                    if newed.account.len() > 0 && newed.pwd.len() > 0 {
                        newed.status = UserInfoStatus::Done;
                        let code = self.user_info.activate_code.clone();
                        let pwd = helper::create_secret_pwd(newed.pwd.to_owned());
                        let body = json!({
                            "account":newed.account.as_str(),
                            "pwd":pwd
                        })
                        .to_string();
                        // 每次新增完都需要执行一次保存操作
                        return Command::perform(
                            async { UserService::create_account(code, body).await },
                            refresh_data,
                        );
                    } else {
                        // 避免增加无用记录(没有名字，也没有密码的账号)
                        self.user_info.users.records.remove(0);
                    }
                }
            }
            UserOpMessage(EUserOpMessage::NewStateCancel) => {
                if self.have_newed() {
                    // 取消新增，则直接删除首元素
                    self.user_info.users.records.remove(0);
                }
            }
            UserOpMessage(EUserOpMessage::Login(account)) => {
                if is_inited_web_driver {
                    let account = self
                        .user_info
                        .users
                        .records
                        .iter()
                        .find(|s| s.id == account)
                        .map(|s| (s.id.clone(), s.account.to_string(), s.pwd.to_string()));
                    // 得等到 webdriver(实际上是使用 chromedriver ) 初始化完成后才能执行登录
                    if let (Some((id, account, pwd))) = account {
                        let pwd = helper::decode_secret_pwd(pwd.as_str());
                        return Command::perform(
                            async move {
                                UserService::get_cookie_eid_fp(id, account.as_str(), pwd.as_str())
                                    .await
                            },
                            |t| match t {
                                Ok(Some(info)) => {
                                    info!(
                                        "登录成功！, {}-{}",
                                        info.account.as_str(),
                                        &info.cookie.as_str()[..=30]
                                    );
                                    return UserOpMessage(EUserOpMessage::LoginFinish(info)).into();
                                }

                                _ => {
                                    return UserOpMessage(EUserOpMessage::LoginFail(format!(
                                        "登录失败"
                                    )))
                                    .into();
                                }
                            },
                        );
                    }
                }
                return Command::none();
            }

            UserOpMessage(EUserOpMessage::LoginFinish(info)) => {
                if let Some(us) = self
                    .user_info
                    .users
                    .records
                    .iter_mut()
                    .find(|s| s.id == info.id)
                {
                    let code = self.user_info.activate_code.clone();
                    let id = us.id;
                    us.cookie = info.cookie;
                    us.cookie_last_update_dt = Some(PKLocal::now());
                    let body = json!({
                        "cookie":us.cookie.as_str(),
                        "eid":info.eid.as_str(),
                        "fp":info.fp.as_str()
                    })
                    .to_string();
                    us.eid = info.eid;
                    us.fp = info.fp;
                    // 每次登录完都需要执行一次保存操作
                    return Command::perform(
                        async move { UserService::update_account(code, id, body).await },
                        |_| UserMessage::Noop.into(),
                    );
                } else {
                    error!("Not found user: {}", info.account.as_str())
                }
            }
            UserOpMessage(EUserOpMessage::LoginFail(msg)) => {
                error!("{}", msg);
            }

            UserOpMessage(EUserOpMessage::NewInputAccount(s)) => {
                if self.have_newed() {
                    let newed = &mut self.user_info.users.records[0];
                    newed.account = s.trim().to_string();
                }
            }
            UserOpMessage(EUserOpMessage::NewInputPwd(s)) => {
                if self.have_newed() {
                    let newed = &mut self.user_info.users.records[0];
                    newed.pwd = s.trim().to_string();
                }
            }
            UserOpMessage(ref msg) => {
                let code = self.user_info.activate_code.clone();
                return self.update_user_info(code, msg);
            }
            Select => {
                self.is_selecting = !self.is_selecting;
                is_select_change = true;
            }
            DeleteSomeStates => {
                if self.is_selecting && self.is_activate {
                    let code = self.user_info.activate_code.clone();
                    let ids = self
                        .user_info
                        .users
                        .records
                        .iter()
                        .filter(|s| s.is_selected)
                        .map(|s| s.id.clone())
                        .collect();
                    return Command::perform(
                        async move { UserService::delete_accounts(code, Some(ids)).await },
                        refresh_data,
                    );
                }
            }
            CookieChecking => {
                info!(
                    "CookieChecking:{},{}",
                    self.user_info.user_id, self.user_info.level
                );
                return Command::batch(
                    self.user_info
                        .users
                        .records
                        .iter_mut()
                        .filter(|us| us.check_valid_cookie())
                        .map(|us| {
                            us.status = UserInfoStatus::CookieChecking;
                            let cookie = us.cookie.clone();
                            let account = us.account.clone();
                            Command::perform(
                                async move { UserService::is_valid_cookie(account, cookie).await },
                                |r| match r {
                                    Ok((account, is_valid)) => {
                                        CookieCheckFinish(account, is_valid).into()
                                    }
                                    Err(e) => {
                                        error!("{:?}", e);
                                        CookieCheckFail.into()
                                    }
                                },
                            )
                        }),
                );
            }

            CookieCheckFinish(account, is_valid) => {
                self.user_info
                    .users
                    .records
                    .iter_mut()
                    .filter(|us| us.account == account.as_str())
                    .map(|us| {
                        us.status = UserInfoStatus::Done;
                        if is_valid {
                            us.cookie_last_update_dt = Some(PKLocal::now());
                        } else {
                            us.cookie_last_update_dt = None;
                        }
                    })
                    .count();
            }
            _ => {
                info!("{:?}", message);
            }
        }
        if is_select_change && !self.is_selecting {
            let msg = EUserOpMessage::Selected(None);
            let code = self.user_info.activate_code.clone();
            return self.update_user_info(code, &msg);
        }
        Command::none()
    }

    pub fn view(&mut self) -> Container<JdMiaoshaAppMessage> {
        let is_selecting = self.is_selecting;
        let (headers, portions) = Self::left_right_portions(is_selecting);

        let states: Element<_> = self
            .user_info
            .users
            .records
            .iter_mut()
            .enumerate()
            .fold(Column::new().spacing(3), |column, (idx, state)| {
                column.push(state.view(idx, is_selecting).map(move |msg| msg.into()))
            })
            .into();
        let search_row = Row::new()
            .align_items(Align::Center)
            .push(Space::with_width(Length::FillPortion(1)))
            .push(
                TextInput::new(
                    &mut self.search_input_state,
                    "账号",
                    self.search_input_txt.as_str(),
                    |s| UserMessage::SearchInput(s).into(),
                )
                .size(24)
                .padding(3)
                .width(Length::Units(120))
                .on_submit(UserMessage::Search.into()),
            )
            .push(
                Button::new(
                    &mut self.search_button_state,
                    Text::new("搜索").horizontal_alignment(HorizontalAlignment::Center),
                )
                .style(style::ActionButton)
                .on_press(UserMessage::Search.into()),
            )
            .push(
                Button::new(
                    &mut self.search_reset_button_state,
                    Text::new("重置").horizontal_alignment(HorizontalAlignment::Center),
                )
                .style(style::ActionButton)
                .on_press(UserMessage::SearchReset.into()),
            )
            .spacing(10)
            .padding(3);
        let scroll = Scrollable::new(&mut self.scroll_state)
            .scrollbar_margin(1)
            .scrollbar_width(1)
            .style(style::ScrollableBarStyle)
            .max_height(super::MAX_SCROLL_HEIGHT)
            .push(Container::new(states).width(Length::Fill).center_x());
        let op_button = |state, txt, msg: UserMessage| {
            Button::new(state, Text::new(txt))
                .style(style::ActionButton)
                .on_press(msg.into())
        };
        let mut global_op_box_row = Row::new().spacing(4).push(op_button(
            &mut self.select_button_state,
            if self.is_selecting {
                "取消"
            } else {
                "选择"
            },
            UserMessage::Select,
        ));
        if self.is_selecting {
            global_op_box_row = global_op_box_row.push(op_button(
                &mut self.delete_button_state,
                "删除",
                UserMessage::DeleteSomeStates,
            ));
        }
        if !self.is_selecting {
            global_op_box_row = global_op_box_row
                .push(Space::with_width(Length::FillPortion(1)))
                .push(op_button(
                    &mut self.cookie_check_button_state,
                    "一键检查登录",
                    UserMessage::CookieChecking,
                ))
                .push(op_button(
                    &mut self.new_button_state,
                    "新增",
                    UserMessage::NewState,
                ))
                .push(op_button(
                    &mut self.import_button_state,
                    "导入",
                    UserMessage::Import,
                ))
                .push(op_button(
                    &mut self.clear_button_state,
                    "清空",
                    UserMessage::ClearAll,
                ));
        }

        let mut left = Container::new(
            Column::new()
                .width(Length::Fill)
                .spacing(4)
                .push(search_row)
                .push(Row::with_children(super::get_headers(&headers, &portions)))
                .push(scroll)
                .push(Rule::horizontal(6).style(style::UserLineRule))
                .push(global_op_box_row),
        );

        let mut right = Column::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(3)
            .align_items(Align::End);

        if !self.is_activate {
            right = right
                .push(
                    Row::new()
                        .push(Container::new(Text::new("激活码:").size(24)).padding(3))
                        .push(Space::with_width(Length::Units(3)))
                        .push(
                            TextInput::new(
                                &mut self.activate_input_state,
                                "请输入激活码",
                                self.user_info.activate_code.as_str(),
                                |txt| UserMessage::ActivateInput(txt).into(),
                            )
                            .on_submit(UserMessage::Activate.into())
                            .size(24)
                            .padding(3),
                        ),
                )
                .push(
                    Row::new().push(
                        Button::new(&mut self.activate_button_state, Text::new("激活"))
                            .style(style::ActionButton)
                            .on_press(UserMessage::Activate.into()),
                    ),
                );
        } else {
            right = right
                .push(
                    Row::new()
                        .push(Text::new("激活码:"))
                        .push(Space::with_width(Length::FillPortion(1)))
                        .push(
                            Text::new(self.user_info.activate_code.as_str())
                                .color(Color::from_rgb8(1, 0, 0)),
                        ),
                )
                .push(
                    Row::new()
                        .push(Text::new("到期时间:"))
                        .push(Space::with_width(Length::FillPortion(1)))
                        .push(Text::new(datetime_fmt(
                            self.user_info
                                .expire_date
                                .as_ref()
                                .unwrap_or(&PKLocal::now()),
                        ))),
                )
                .push(
                    Row::new()
                        .push(Text::new("信 息:"))
                        .push(Space::with_width(Length::FillPortion(1)))
                        .push(Text::new(UserInfo::version_level(self.user_info.level))),
                );
        }
        return Container::new(
            Row::new()
                .push(Container::new(left).width(Length::FillPortion(4)))
                .push(Rule::vertical(4).style(style::UserLineRule))
                .push(
                    Container::new(right)
                        .width(Length::FillPortion(1))
                        .align_y(Align::Center),
                )
                .width(Length::Fill),
        )
        .width(Length::Fill);
    }
}

fn show_short_cookie(cookie: &str) -> &str {
    if cookie.len() > 0 {
        &cookie[..=10]
    } else {
        "???"
    }
}

fn op_check_box_button<'a>(
    state: &'a mut button::State,
    is_selected: bool,
    id: IDType,
    msg: &str,
) -> Element<'a, UserMessage> {
    Row::new()
        .push(
            Button::new(state, Text::new(if is_selected { "•" } else { "" }))
                .width(Length::Units(20))
                .height(Length::Units(20))
                .on_press(EUserOpMessage::Selected(Some(id)).into())
                .style(if is_selected {
                    style::CheckBoxButton::Checked
                } else {
                    style::CheckBoxButton::UnChecked
                }),
        )
        .push(Space::with_width(Length::Units(10)))
        .push(Text::new(msg).color(TXT_COLOR))
        .into()
}
const TXT_COLOR: Color = Color::from_rgb(0.0, 191.0 / 255.0, 1.0);

fn label_txt(
    txt: &str,
    portion: u16,
    align_x: Option<HorizontalAlignment>,
) -> Element<UserMessage> {
    Text::new(txt)
        .horizontal_alignment(align_x.unwrap_or(HorizontalAlignment::Center))
        .vertical_alignment(VerticalAlignment::Center)
        .color(TXT_COLOR)
        .width(Length::FillPortion(portion))
        .into()
}

fn label_txt_input<'a, F>(
    state: &'a mut text_input::State,
    placeholder: &str,
    txt: &str,
    portion: u16,
    is_password: bool,
    on_change: F,
) -> Element<'a, UserMessage>
where
    F: 'static + Fn(String) -> UserMessage,
{
    let mut input = TextInput::new(state, placeholder, txt, on_change)
        .width(Length::FillPortion(portion))
        // .size(24)
        .padding(3);
    if is_password {
        input = input.password()
    }
    input.into()
}

impl UserState {
    fn new() -> Self {
        Default::default()
    }

    fn check_valid_cookie(&self) -> bool {
        !Self::maybe_valid_cookie(self.cookie_last_update_dt.clone())
    }

    fn cookie_last_update_dt_str(&self) -> String {
        if let Some(ref dt) = self.cookie_last_update_dt {
            datetime_fmt(dt.clone())
        } else {
            "".to_string()
        }
    }
    pub fn maybe_valid_cookie(dt: Option<PKDateTime>) -> bool {
        let mut is_valid = false;
        if let Some(dt) = dt {
            let now = PKLocal::now();
            if now.timestamp() - dt.timestamp() <= 10 * 60 {
                is_valid = true;
            }
        }
        is_valid
    }

    pub fn maybe_valid_cookie2(&self) -> bool {
        Self::maybe_valid_cookie(self.cookie_last_update_dt.clone())
    }

    fn filter(&self, msg: &EUserOpMessage) -> bool {
        use EUserOpMessage::*;
        let mut ret = false;
        match msg {
            ViewPwd(ref _account)
            | UpdateState(ref _account)
            | DeleteState(ref _account)
            | UpdateState(ref _account)
            | Selected(Some(ref _account)) => {
                ret = &self.id == _account;
            }
            Selected(None) => {
                ret = true;
            }
            EditInputPwd(_) | EditInputAccount(_) => {
                ret = self.status == UserInfoStatus::Editing;
            }
            EditStateCancel(ref _account) | EditStateFinish(ref _account) => {
                ret = &self.id == _account && self.status == UserInfoStatus::Editing;
            }
            _ => {
                ret = true;
            }
        }
        return ret;
    }

    fn update(&mut self, code: String, msg: &EUserOpMessage) -> Command<UserMessage> {
        use EUserOpMessage::*;
        if !self.filter(msg) {
            return Command::none();
        }
        info!("account{}, {:?}", self.account.as_str(), msg,);
        let refresh_data = |t: crate::error::Result<_>| {
            if t.is_ok() {
                UserMessage::SearchReset
            } else {
                UserMessage::Noop
            }
        };
        match msg {
            EditStateCancel(_) => {
                if self.status == UserInfoStatus::Editing {
                    self.status = UserInfoStatus::Done;
                }
            }
            EditStateFinish(_) => {
                if self.status == UserInfoStatus::Editing {
                    self.status = UserInfoStatus::Done;
                    let pwd = helper::create_secret_pwd(self.pwd.to_owned());
                    let body = json!({
                        "account":self.account.as_str(),
                        "pwd":pwd,
                    })
                    .to_string();
                    let id = self.id;
                    return Command::perform(
                        async move { UserService::update_account(code, id, body).await },
                        refresh_data,
                    );
                }
            }

            ViewPwd(_) => {
                self.is_view_passwd = !self.is_view_passwd;
            }
            Selected(os) => {
                if let Some(_) = os {
                    // Some 表示选中某个
                    self.is_selected = !self.is_selected;
                } else {
                    // None 时是清除所有
                    self.is_selected = false;
                }
            }
            UpdateState(s) => {
                if self.id.eq(s) && self.status != UserInfoStatus::NewIng {
                    self.status = UserInfoStatus::Editing;
                }
            }
            EditInputPwd(s) => {
                let arg = s.trim().to_string();
                if self.pwd != arg {
                    self.pwd = arg;
                }
            }
            EditInputAccount(s) => {
                let arg = s.trim().to_string();
                if self.account != arg {
                    self.account = arg;
                }
            }
            _ => {}
        }
        return Command::none();
    }

    fn view(&mut self, idx: usize, is_selecting: bool) -> Element<UserMessage> {
        let mut row = Row::new().align_items(Align::Center);
        let mut op_box_row = Row::new()
            .push(Space::with_width(Length::Units(3)))
            .spacing(3)
            .align_items(Align::Center)
            .width(Length::Fill);
        let op_button =
            |state, button_name, len: Length, msg: EUserOpMessage| -> Element<UserMessage> {
                Button::new(
                    state,
                    Text::new(button_name).horizontal_alignment(HorizontalAlignment::Center),
                )
                .on_press(msg.into())
                .width(len)
                .style(style::ActionButton)
                .into()
            };
        let cookie_image = Row::new()
            .width(Length::FillPortion(UserComponent::APP_CK_PORTION))
            .push(label_txt(show_short_cookie(self.cookie.as_str()), 3, None))
            .push(Space::with_width(Length::FillPortion(PORTION_1)))
            .push(if self.status == UserInfoStatus::CookieChecking {
                img::loading_image()
            } else {
                if Self::maybe_valid_cookie(self.cookie_last_update_dt.clone()) {
                    img::right_image()
                } else {
                    img::wrong_image()
                }
            })
            .push(Space::with_width(6.into()));
        let opbuttons = match self.status {
            UserInfoStatus::NewIng | UserInfoStatus::Editing => {
                let status = self.status;
                let account_msg_fn = move |s| {
                    if status == UserInfoStatus::NewIng {
                        EUserOpMessage::NewInputAccount(s).into()
                    } else {
                        EUserOpMessage::EditInputAccount(s).into()
                    }
                };
                let pwd_msg_fn = move |s| {
                    if status == UserInfoStatus::NewIng {
                        EUserOpMessage::NewInputPwd(s).into()
                    } else {
                        EUserOpMessage::EditInputPwd(s).into()
                    }
                };
                row = row
                    .push(
                        Container::new(Row::new().push(Space::with_width(30.into())).push(
                            label_txt_input(
                                &mut self.account_input_state,
                                "请输入账号",
                                self.account.as_str(),
                                PORTION_1,
                                false,
                                account_msg_fn,
                            ),
                        ))
                        .width(Length::FillPortion(UserComponent::ACCOUNT_PORTION)),
                    )
                    .push(label_txt_input(
                        &mut self.pwd_input_state,
                        "请输入密码",
                        self.pwd.as_str(),
                        UserComponent::PWD_PORTION,
                        true,
                        pwd_msg_fn,
                    ));

                row = row.push(cookie_image);
                vec![
                    op_button(
                        &mut self.update_button_state,
                        "确定",
                        Length::FillPortion(PORTION_1),
                        if self.status == UserInfoStatus::NewIng {
                            EUserOpMessage::NewStateFinish
                        } else {
                            EUserOpMessage::EditStateFinish(self.id)
                        },
                    ),
                    Space::with_width(Length::FillPortion(PORTION_1)).into(),
                    op_button(
                        &mut self.delete_button_state,
                        "取消",
                        Length::FillPortion(PORTION_1),
                        if self.status == UserInfoStatus::NewIng {
                            EUserOpMessage::NewStateCancel
                        } else {
                            EUserOpMessage::EditStateCancel(self.id)
                        },
                    ),
                ]
            }
            _ => {
                let account_col: Element<UserMessage> = if is_selecting {
                    Container::new(op_check_box_button(
                        &mut self.check_button_state,
                        self.is_selected.to_owned(),
                        self.id,
                        self.account.as_str(),
                    ))
                    .align_x(Align::Start)
                    .width(Length::FillPortion(UserComponent::ACCOUNT_PORTION))
                    .into()
                } else {
                    Container::new(
                        Row::new()
                            .push(Space::with_width(30.into()))
                            .push(label_txt(
                                self.account.as_str(),
                                PORTION_1,
                                Some(HorizontalAlignment::Left),
                            ))
                            .width(Length::Fill),
                    )
                    .width(Length::FillPortion(UserComponent::ACCOUNT_PORTION))
                    .into()
                };
                let pwd = if self.is_view_passwd {
                    helper::decode_secret_pwd(self.pwd.as_str())
                } else {
                    "****".to_string()
                };
                row = row
                    .push(account_col)
                    .push(op_button(
                        &mut self.view_pwd_button_state,
                        pwd.as_str(),
                        Length::FillPortion(UserComponent::PWD_PORTION),
                        EUserOpMessage::ViewPwd(self.id),
                    ))
                    .push(cookie_image);
                if !is_selecting {
                    let mut r = Vec::with_capacity(3);
                    // if Self::maybe_valid_cookie(self.cookie_last_update_dt.clone()) {
                    //     r.push(Space::with_width(Length::FillPortion(1)).into());
                    // } else {
                    //     r.push(op_button(
                    //         &mut self.login_button_state,
                    //         "登录",
                    //         Length::FillPortion(1),
                    //         EUserOpMessage::Login(self.id),
                    //     ));
                    // }
                    r.push(op_button(
                        &mut self.login_button_state,
                        "登录",
                        Length::FillPortion(1),
                        EUserOpMessage::Login(self.id),
                    ));
                    r.append(&mut vec![
                        op_button(
                            &mut self.update_button_state,
                            "修改",
                            Length::FillPortion(1),
                            EUserOpMessage::UpdateState(self.id),
                        ),
                        op_button(
                            &mut self.delete_button_state,
                            "删除",
                            Length::FillPortion(1),
                            EUserOpMessage::DeleteState(self.id),
                        ),
                    ]);
                    r
                } else {
                    vec![]
                }
            }
        };

        for opb in opbuttons.into_iter() {
            op_box_row = op_box_row.push(opb);
        }
        op_box_row = op_box_row.push(Space::with_width(Length::Units(3)));

        Container::new(
            row.push(
                Container::new(op_box_row).width(Length::FillPortion(UserComponent::OP_PORTION)),
            ),
        )
        .width(Length::Fill)
        .into()
    }
}
