use std::cmp::max;
use std::collections::{HashMap, HashSet};
use std::env;
use std::ops::{Deref, Sub};
use std::pin::Pin;
use std::sync::Arc;

use iced::*;
use iced::pane_grid::Line;
use iced_wgpu::wgpu::Label;
use log::{debug, error, info, set_max_level, warn};
use rand::Rng;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde_json::{json, to_string};
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant, Sleep, timeout};

use crate::{IDType, NORMAL, PKDate, PKDateTime, PKLocal, YUYUE};
use crate::models::*;
use crate::services::shopping_cart::ShoppingCartService;
use crate::ui::ShoppingCartMessage::Noop;
use crate::utils::*;
use crate::utils::icon::{plus_icon, sub_icon, time_clock_icon};

use super::PORTION_1;
use super::style;
use super::super::*;

type YuyueSleeps = Arc<RwLock<HashMap<IDType, (Instant, i64)>>>;
lazy_static! {
    static ref SHOPPING_CARTS_YUYUE_SLEEPS: YuyueSleeps = YuyueSleeps::default();
}

async fn set_yuyue_sleep(id: IDType, deadline_timestamp_millis: i64, ins: Instant) -> bool {
    let mut guard = SHOPPING_CARTS_YUYUE_SLEEPS.write().await;
    if let Some(entry) = guard.get_mut(&id) {
        if entry.1 == deadline_timestamp_millis {
            return false;
        }
        *entry = (ins, deadline_timestamp_millis);
    } else {
        guard.insert(id, (ins, deadline_timestamp_millis));
    }
    true
}

async fn remove_yuyue_sleep(id: IDType) {
    let mut guard = SHOPPING_CARTS_YUYUE_SLEEPS.write().await;
    guard.remove(&id);
}

async fn yuyue_sleep(id: IDType, deadline_timestamp_millis: i64, ins: Instant) -> bool {
    let changed = set_yuyue_sleep(id, deadline_timestamp_millis, ins).await;
    if changed {
        let ins = {
            let guard = SHOPPING_CARTS_YUYUE_SLEEPS.read().await;
            guard.get(&id).unwrap().0.clone()
        };
        sleep_until(ins).await;
    }
    changed
}

#[derive(Default)]
pub struct ShoppingCartComponent {
    // 商品搜索输入框的内容
    // 滚动条
    is_selecting: bool,
    scroll_state: scrollable::State,
    // 选择
    select_button_state: button::State,
    // 立即购买
    immediately_purchase_button_state: button::State,
    // 预约购买
    // timeout_purchase_button_state: button::State,
    // 删除
    delete_button_state: button::State,
    // 清空
    clear_button_state: button::State,

    prev_page_button_state: button::State,
    // 清空
    next_page_button_state: button::State,
    page_no: i64,
    has_prev_page: bool,
    has_next_page: bool,

    // 搜索
    search_input_txt: String,
    search_input_state: text_input::State,
    search_button_state: button::State,
    search_reset_button_state: button::State,
    // 激活码
    pub activate_code: String,
    // 购物车信息
    pub prods: ShoppingCartPageState,
}

impl ShoppingCartComponent {
    pub const NAME_PORTION: u16 = 4;
    pub const SKU_PORTION: u16 = 2;
    pub const PRICE_PORTION: u16 = 2;
    pub const STOCK_PORTION: u16 = 1;
    pub const TIMEOUT_PORTION: u16 = 3;
    pub const AMOUNT_PORTION: u16 = 1;
    pub const OP_PORTION: u16 = 3;

    pub fn new() -> Self {
        let mut s = ShoppingCartComponent::default();
        s.page_no = 1;
        s
    }

    fn header_portions() -> (Vec<String>, Vec<u16>) {
        (
            vec![
                "商品名称".to_string(),
                "商品SKU".to_string(),
                "价格".to_string(),
                "库存".to_string(),
                "数量".to_string(),
                "预约时间".to_string(),
                "操作".to_string(),
            ],
            vec![
                Self::NAME_PORTION,
                Self::SKU_PORTION,
                Self::PRICE_PORTION,
                Self::STOCK_PORTION,
                Self::AMOUNT_PORTION,
                Self::TIMEOUT_PORTION,
                Self::OP_PORTION,
            ],
        )
    }

    fn monitor_goods_stock(&self) -> Command<JdMiaoshaAppMessage> {
        Command::batch(
            self.prods
                .records
                .iter()
                .filter(|item| item.is_stock == 0)
                .map(|item| {
                    let sku = item.sku.clone();
                    let id = item.id.clone();
                    Command::perform(async move { (id, sku) }, |(id, sku)| {
                        // 检查商品库存的消息
                        GoodsMessage::MonitorStock(id, sku).into()
                    })
                }),
        )
    }

    fn get_goods_coupons(&self) -> Command<JdMiaoshaAppMessage> {
        Command::batch(
            self.prods
                .records
                .iter()
                .filter(|item| item.purchase_status == "ready")
                .map(|item| {
                    let sku = item.sku.clone();
                    Command::perform(async move { sku }, |sku| UserMessage::GetCoupon(sku).into())
                }),
        )
    }

    fn yuyue_goods_commands(&mut self, id: Option<IDType>) -> Command<JdMiaoshaAppMessage> {
        // 只触发自动购买那些状态为 ready 且设置了预约时间小于当前时间的商品
        let records = &self.prods.records;
        let iter = records.iter().filter(|r| {
            if let Some(id) = &id {
                r.id == *id
            } else {
                true
            }
        });
        let mut cmds = iter
            .map(|record| {
                let mut yuyue_timeout = 0;
                let now = now();
                let instant_now = tokio::time::Instant::now();
                let id = record.id;
                let mut deadline_timestamp_millis = 0;
                if (record.purchase_type.eq(YUYUE) && record.purchase_status.eq("yuyueing"))
                    || (!record.purchase_type.eq(YUYUE) && record.purchase_status.eq("ready"))
                {
                    if let Some(yuyue_dt) = &record.yuyue_dt {
                        deadline_timestamp_millis = yuyue_dt.timestamp_millis();
                        yuyue_timeout = deadline_timestamp_millis - now.timestamp_millis();
                        if yuyue_timeout > 0 {
                            yuyue_timeout -= ahead_purchase_millis(); // 定时的时候，提前一定时间就开始抢了
                            yuyue_timeout = max(1, yuyue_timeout);
                            deadline_timestamp_millis -= yuyue_timeout;
                            info!(
                                "定时预约购买:{}-{}-{}-{}-[{}-{}, {}]",
                                id,
                                record.sku,
                                record.purchase_type,
                                record.purchase_status,
                                datetime_fmt(&now),
                                datetime_fmt(yuyue_dt),
                                yuyue_timeout,
                            );
                        } else {
                            // 时间设置错误
                            yuyue_timeout = 0;
                        }
                    }
                } else if record.purchase_type.eq(YUYUE) && record.purchase_status == "ready" {
                    return Command::perform(async {}, move |_| {
                        ShoppingCartMessage::SubmitOrder(id).into()
                    });
                }
                let yuyue_timeout = yuyue_timeout as u64;
                if yuyue_timeout > 0 {
                    let ins = instant_now + Duration::from_millis(yuyue_timeout);
                    Command::perform(
                        yuyue_sleep(id, deadline_timestamp_millis, ins),
                        move |changed| {
                            if changed {
                                ShoppingCartMessage::SubmitOrder(id).into()
                            } else {
                                ShoppingCartMessage::Noop.into()
                            }
                        },
                    )
                } else {
                    Command::none()
                }
            })
            .collect::<Vec<Command<_>>>();
        {
            let mut rng = thread_rng();
            cmds.shuffle(&mut rng);
        }
        return Command::batch(cmds);
    }

    pub fn update(&mut self, message: ShoppingCartMessage) -> Command<JdMiaoshaAppMessage> {
        use ShoppingCartMessage::*;
        let msg = message.clone();
        match message {
            Loading => {
                if self.activate_code.is_empty() {
                    self.has_prev_page = false;
                    self.has_next_page = false;
                    self.prods = ShoppingCartPageState::default();
                    return Command::none();
                }
                let code = self.activate_code.clone();
                let page_no = self.prods.page_no.clone();
                let key_word = self.search_input_txt.clone();
                return Command::batch(vec![
                    Command::perform(
                        async move {
                            ShoppingCartService::list_cart_goods(code, key_word, page_no).await
                        },
                        |r| match r {
                            Ok(rdata) => LoadFinish(rdata).into(),
                            Err(e) => {
                                error!("{:?}", e);
                                LoadFail.into()
                            }
                        },
                    ),
                    Command::perform(async {}, |_| {
                        UserMessage::AddToCartAndLoadCartGoodsSkuUuids(0, None).into()
                    }),
                ]);
            }
            LoadFinish(data) => {
                if let Some(d) = data {
                    self.has_prev_page = d.page_no > 1 && d.page_no <= d.pages;
                    self.has_next_page = d.page_no < d.pages;
                    self.prods = d;
                    let yuyue_cmds = self.yuyue_goods_commands(None);
                    let mut cmds = vec![
                        // 只触发自动购买那些状态为 ready 且设置了预约时间小于当前时间的商品
                        self.get_goods_coupons(),
                        yuyue_cmds,
                    ];
                    return Command::batch(cmds);
                } else {
                    warn!("数据加载失败-LoadFinish");
                }
            }

            YuyueChecking(id) => {
                return self.yuyue_goods_commands(Some(id));
            }
            StockChecking => {
                return self.monitor_goods_stock();
            }

            LoadFail => {
                warn!("数据加载失败-LoadFail");
            }
            PrevPage | NextPage => {
                self.prods.page_no = match message {
                    PrevPage => std::cmp::max(self.prods.page_no - 1, 1),
                    _ => std::cmp::min(self.prods.page_no + 1, self.prods.pages),
                };
                if self.activate_code.is_empty() {
                    return Command::none();
                }
                return Command::perform(async move {}, |_| ShoppingCartMessage::Loading.into());
            }
            SearchInput(s) => {
                self.search_input_txt = s.trim().to_string();
            }
            Search => {
                // 重置搜索的话会把页号 置  1
                self.prods.page_no = 1;
                if self.activate_code.is_empty() {
                    return Command::none();
                }
                return Command::perform(async move {}, |_| ShoppingCartMessage::Loading.into());
            }
            SearchReset => {
                // 重置搜索的话会把页号置  1, 并把 搜索内容清空
                self.prods.page_no = 1;
                self.search_input_txt.clear();
                if self.activate_code.is_empty() {
                    return Command::none();
                }
                return Command::perform(async move {}, |_| ShoppingCartMessage::Loading.into());
            }

            Editing(_id)
            | EditInputTimeoutDt(_id, _)
            | EditFinish(_id)
            | UpdateStock(_id, _)
            | EditCancel(_id)
            | SelectOne(_id)
            | ImmediatelyPurchase(_id)
            | SubNum(_id)
            | AddNum(_id) => {
                if self.activate_code.is_empty() {
                    return Command::none();
                }
                let code = self.activate_code.clone();
                return Command::batch(
                    self.prods
                        .records
                        .iter_mut()
                        .filter(|p| p.id == _id)
                        .map(|p| p.update(code.clone(), &msg).map(|m| m.into())),
                );
            }
            SubmitOrderFinish(_id, status) => {
                if self.activate_code.is_empty() {
                    return Command::none();
                }
                let mut cmds = vec![];
                if !status.eq("yuyueing") {
                    // 不需要管返回结果，定时购买执行完了之后，需要从定时任务集里面删掉
                    cmds.push(Command::perform(remove_yuyue_sleep(_id), |_| {
                        ShoppingCartMessage::Noop.into()
                    }));
                }
                let code = self.activate_code.clone();
                cmds.extend(
                    self.prods
                        .records
                        .iter_mut()
                        .filter(|p| p.id == _id)
                        .map(|p| p.update(code.clone(), &msg).map(|m| m.into())),
                );
                return Command::batch(cmds);
            }
            DeleteOneState(_id) => {
                if self.activate_code.is_empty() {
                    return Command::none();
                }
                let code = self.activate_code.clone();
                return Command::perform(
                    async move { ShoppingCartService::delete_one_cart_goods(code, _id).await },
                    |r| match r {
                        Ok(_) => Loading.into(),
                        Err(e) => {
                            error!("{:?}", e);
                            Noop.into()
                        }
                    },
                );
            }
            DeleteSomeStates | ClearAll => {
                if self.activate_code.is_empty() {
                    return Command::none();
                }
                let ids = match message {
                    ClearAll => None,
                    DeleteSomeStates => Some(
                        self.prods
                            .records
                            .iter()
                            .filter(|p| p.is_selected)
                            .map(|u| u.id.clone())
                            .collect(),
                    ),
                    _ => unreachable!(),
                };
                let code = self.activate_code.clone();
                return Command::perform(
                    async move { ShoppingCartService::delete_cart_goods(code, ids).await },
                    |r| match r {
                        Ok(kw) => Loading.into(),
                        Err(e) => {
                            error!("{:?}", e);
                            Noop.into()
                        }
                    },
                );
            }

            Select => {
                self.is_selecting = !self.is_selecting;
                if !self.is_selecting {
                    self.prods
                        .records
                        .iter_mut()
                        .filter(|p| p.is_selected)
                        .map(|p| p.is_selected = false)
                        .count();
                }
            }
            _ => {}
        }
        // 记录是否点击了选择按钮，避免发送过多的消息
        Command::none()
    }

    pub fn view(&mut self) -> Container<JdMiaoshaAppMessage> {
        let is_selecting = self.is_selecting;
        let (headers, portions) = Self::header_portions();

        let states: Element<_> = self
            .prods
            .records
            .iter_mut()
            .enumerate()
            .fold(Column::new().spacing(3), |column, (idx, state)| {
                column.push(state.view(idx, is_selecting).map(move |msg| msg.into()))
            })
            .into();
        let mut search_row = Row::new().align_items(Align::Center);

        if self.prods.pages > 0 {
            let pages = Text::new(format!(
                "第{}页/共{}页",
                self.prods.page_no, self.prods.pages
            ))
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center)
            .size(24)
            .color(TXT_COLOR);
            search_row = search_row.push(pages);
        }
        search_row = search_row
            .push(Space::with_width(Length::FillPortion(1)))
            .push(
                TextInput::new(
                    &mut self.search_input_state,
                    "商品名|SKU",
                    self.search_input_txt.as_str(),
                    |s| ShoppingCartMessage::SearchInput(s).into(),
                )
                .size(24)
                .padding(3)
                .width(Length::Units(120))
                .on_submit(ShoppingCartMessage::Search.into()),
            )
            .push(
                Button::new(
                    &mut self.search_button_state,
                    Text::new("搜索").horizontal_alignment(HorizontalAlignment::Center),
                )
                .style(style::ActionButton)
                .on_press(ShoppingCartMessage::Search.into()),
            )
            .push(
                Button::new(
                    &mut self.search_reset_button_state,
                    Text::new("重置").horizontal_alignment(HorizontalAlignment::Center),
                )
                .style(style::ActionButton)
                .on_press(ShoppingCartMessage::SearchReset.into()),
            )
            .spacing(10)
            .padding(3);
        let scroll = Scrollable::new(&mut self.scroll_state)
            .scrollbar_margin(1)
            .scrollbar_width(1)
            .style(style::ScrollableBarStyle)
            .max_height(super::MAX_SCROLL_HEIGHT)
            .push(Container::new(states).width(Length::Fill).center_x());
        let op_button = |state, txt, msg: ShoppingCartMessage| {
            Button::new(state, Text::new(txt))
                .style(style::ActionButton)
                .on_press(msg.into())
        };
        let mut global_op_box_row = Row::new().spacing(4);

        global_op_box_row = global_op_box_row.push(op_button(
            &mut self.select_button_state,
            if self.is_selecting {
                "取消"
            } else {
                "选择"
            },
            ShoppingCartMessage::Select,
        ));
        let prev_page_button = op_button(
            &mut self.prev_page_button_state,
            "上一页",
            ShoppingCartMessage::PrevPage,
        );
        let delete_button = op_button(
            &mut self.delete_button_state,
            "删除",
            ShoppingCartMessage::DeleteSomeStates,
        );

        if self.is_selecting {
            global_op_box_row = global_op_box_row.push(delete_button);
            // .push(op_button(
            //     &mut self.immediately_purchase_button_state,
            //     "立即购买",
            //     ShoppingCartMessage::ImmediatelySomeStates,
            // ));
        }
        if self.has_prev_page {
            global_op_box_row = global_op_box_row.push(prev_page_button);
        }
        global_op_box_row =
            global_op_box_row.push(Space::with_width(Length::FillPortion(PORTION_1)));

        if self.has_next_page {
            global_op_box_row = global_op_box_row.push(op_button(
                &mut self.next_page_button_state,
                "下一页",
                ShoppingCartMessage::NextPage,
            ));
        }
        global_op_box_row = global_op_box_row.push(op_button(
            &mut self.clear_button_state,
            "清空",
            ShoppingCartMessage::ClearAll,
        ));

        return Container::new(
            Column::new()
                .width(Length::Fill)
                .spacing(4)
                .push(search_row)
                .push(Row::with_children(super::get_headers(&headers, &portions)))
                .push(scroll)
                .push(Rule::horizontal(6).style(style::UserLineRule))
                .push(global_op_box_row),
        )
        .width(Length::Fill);
    }
}

fn op_check_box_button<'a, 'b>(
    state: &'a mut button::State,
    is_selected: bool,
    txt: &'b str,
    id: IDType,
) -> Element<'a, ShoppingCartMessage> {
    Row::new()
        .push(
            Button::new(state, Text::new(if is_selected { "•" } else { "" }))
                .width(Length::Units(20))
                .height(Length::Units(20))
                .on_press(ShoppingCartMessage::SelectOne(id))
                .style(if is_selected {
                    style::CheckBoxButton::Checked
                } else {
                    style::CheckBoxButton::UnChecked
                }),
        )
        .push(Space::with_width(Length::Units(10)))
        .push(Text::new(txt).color(TXT_COLOR))
        .align_items(Align::Center)
        .into()
}
const TXT_COLOR: Color = Color::from_rgb(0.0, 191.0 / 255.0, 1.0);

fn label_txt<'a, T: AsRef<str>>(
    txt: T,
    portion: u16,
    align_x: Option<HorizontalAlignment>,
    color: Option<Color>,
) -> Element<'a, ShoppingCartMessage> {
    Text::new(txt.as_ref())
        .horizontal_alignment(align_x.unwrap_or(HorizontalAlignment::Center))
        .vertical_alignment(VerticalAlignment::Center)
        .color(color.unwrap_or(TXT_COLOR))
        .width(Length::FillPortion(portion))
        .into()
}

impl CartProdState {
    fn update(&mut self, code: String, msg: &ShoppingCartMessage) -> Command<ShoppingCartMessage> {
        use ShoppingCartMessage::*;

        let update_cart_goods = |code, id, body, mt: u8| {
            Command::perform(
                async move { ShoppingCartService::update_cart_goods(code, id, body).await },
                move |t| {
                    if let Err(e) = t {
                        error!("{:?}", e);
                    }
                    if mt == 1 {
                        YuyueChecking(id)
                    } else if mt == 2 {
                        Loading
                    } else {
                        Noop
                    }
                },
            )
        };
        match msg {
            EditCancel(_) => {
                if self.status == CartProdStatus::Editing {
                    self.status = CartProdStatus::Done;
                }
            }
            SelectOne(_) => {
                self.is_selected = !self.is_selected;
            }
            Editing(_) => {
                self.status = CartProdStatus::Editing;
            }
            EditInputTimeoutDt(_, s) => {
                self.yuyue_txt = s.to_owned();
            }

            UpdateStock(_id, _stock_status) => {
                // 把该商品设为有货状态
                self.is_stock = 1;
                let id = _id.clone();
                let code = code.to_owned();
                // 在数据库存为有货状态
                let body = json!({
                    "op":8,
                    "yuyue_dt":null,
                })
                    .to_string();
                // 执行立即购买和 在数据库更新为有货状态
                let mut cmds = vec![update_cart_goods(code, id, body, 0)];
                if self.yuyue_dt.is_none() && self.purchase_status != "purchasing" {
                    // 避免无条件的执行立即购买
                    cmds.push(Command::perform(async move {}, move |_| {
                        ImmediatelyPurchase(id).into()
                    }));
                }
                return Command::batch(cmds);
            }

            SubmitOrderFinish(_id, status) => {
                let mut op = 0;
                self.cur_check += 1;
                if status == &"success" {
                    // 只要有一个抢购成功， 那就是成功了
                    op = 5;
                    self.purchase_status = "success".to_string()
                } else if status == &"yuyueing" {
                    // 预约中
                    op = 7;
                    self.purchase_status = "yuyueing".to_string()
                } else {
                    let max_check = JdMiaoshaApp::get_check_times(self.id);
                    info!(
                        "id:{},cur_check:{},max_check:{}",
                        self.id, self.cur_check, max_check
                    );
                    if self.cur_check >= max_check && self.purchase_status.eq("purchasing") {
                        // 所有的抢购都没成功，那就是抢购失败了, 且状态还是为 purchasing 才修改为失败状态
                        op = 6;
                        self.purchase_status = "fail".to_string()
                    }
                }
                if op > 0 {
                    self.purchase_status = status.to_string();
                    let body = json!({
                        "op":op,
                        "yuyue_dt":null,
                    })
                        .to_string();
                    let id = _id.clone();
                    let mut cmds = vec![update_cart_goods(code, id, body, 0)];
                    if op == 5 || op == 6 {
                        cmds.push(Command::perform(async {}, move |_| {
                            MaybeRemoveShoppingCartLock(id).into()
                        }));
                    }
                    return Command::batch(cmds);
                }
            }

            EditFinish(_) => {
                self.yuyue_dt = parse_datetime(self.yuyue_txt.trim());
                if self.status == CartProdStatus::Editing {
                    self.status = CartProdStatus::Done;
                    if self.yuyue_dt.is_some() {
                        let id = self.id.clone();
                        let body = json!({
                            "op":4,
                            "yuyue_dt":datetime_fmt(self.yuyue_dt.as_ref().unwrap()),
                        })
                        .to_string();
                        return update_cart_goods(code, id, body, 1);
                    }
                }
            }

            SubNum(_) => {
                if self.purchase_num > 1 {
                    self.purchase_num -= 1;
                }
                let id = self.id.clone();
                let body = json!({
                    "op":2,
                    "yuyue_dt":null,
                })
                .to_string();
                return update_cart_goods(code, id, body, 0);
            }
            AddNum(_) => {
                self.purchase_num = match self.purchase_num.checked_add(1) {
                    Some(v) => v,
                    None => u32::MAX,
                };

                let id = self.id.clone();
                let body = json!({
                    "op":1,
                    "yuyue_dt":null,
                })
                .to_string();
                return update_cart_goods(code, id, body, 0);
            }

            ImmediatelyPurchase(_) => {
                if self.is_stock == 0 {
                    // 无货则啥都不做
                    warn!("商品:{}{}无货,略过!", self.id, self.sku);
                    return Command::none();
                }
                let id = self.id.clone();
                let body = json!({
                    "op":3,
                    "yuyue_dt":null,
                })
                .to_string();
                let cmd = Command::perform(async move {}, move |_| SubmitOrder(id).into());
                return Command::batch(vec![cmd, update_cart_goods(code, id, body, 2)]);
            }

            _ => {
                info!("{:?}", msg);
            }
        }
        return Command::none();
    }

    fn view(&mut self, idx: usize, is_selecting: bool) -> Element<ShoppingCartMessage> {
        let mut row = Row::new().align_items(Align::Center);
        let mut op_box_row = Row::new()
            .push(Space::with_width(Length::Units(3)))
            .spacing(3)
            .align_items(Align::Center)
            .width(Length::Fill);
        let op_button = |state,
                         button_name,
                         len: Length,
                         msg: ShoppingCartMessage|
         -> Element<ShoppingCartMessage> {
            Button::new(
                state,
                Text::new(button_name).horizontal_alignment(HorizontalAlignment::Center),
            )
            .on_press(msg)
            .width(len)
            .style(style::ActionButton)
            .into()
        };
        let name_col: Element<_> = if is_selecting {
            Container::new(op_check_box_button(
                &mut self.check_button_state,
                self.is_selected.to_owned(),
                self.name.as_str(),
                self.id,
            ))
            .width(Length::FillPortion(ShoppingCartComponent::NAME_PORTION))
            .into()
        } else {
            let ele: Element<ShoppingCartMessage>;
            if self.purchase_status.eq("purchasing") {
                ele = img::loading_image().into();
            } else if self.purchase_status.eq("success") {
                ele = img::right_image().into();
            } else if self.purchase_status.eq("fail") {
                ele = img::wrong_image().into();
            } else if self.purchase_status.eq("yuyueing")
                || self.purchase_status.eq("ready") && self.yuyue_dt.is_some()
            {
                ele = icon::time_clock_icon().into();
            } else {
                ele = Space::with_width(20.into()).into()
            }
            Container::new(
                Row::new()
                    .push(ele)
                    .push(Space::with_width(Length::Units(10)))
                    .push(label_txt(
                        &self.name,
                        PORTION_1,
                        Some(HorizontalAlignment::Left),
                        None,
                    ))
                    .align_items(Align::Center)
                    .width(Length::Fill),
            )
            .width(Length::FillPortion(ShoppingCartComponent::NAME_PORTION))
            .into()
        };

        row = row
            .push(name_col)
            .push(label_txt(
                self.sku.as_str(),
                ShoppingCartComponent::SKU_PORTION,
                None,
                None,
            ))
            .push(label_txt(
                format!("{}/{}", &self.cur_price, &self.ori_price),
                ShoppingCartComponent::PRICE_PORTION,
                None,
                None,
            ))
            .push(label_txt(
                if self.is_stock == 1 {
                    "有货"
                } else {
                    "无货"
                },
                ShoppingCartComponent::STOCK_PORTION,
                None,
                if self.is_stock == 1 {
                    None
                } else {
                    Some(Color::from_rgb8(255, 0, 0))
                },
            ))
            .push(
                Container::new(
                    Row::new()
                        .align_items(Align::Center)
                        // .push(
                        //     Button::new(&mut self.sub_num_button_state, sub_icon())
                        //         .style(style::ActionButton)
                        //         .on_press(ShoppingCartMessage::SubNum(self.id))
                        //         .width(Length::FillPortion(PORTION_1)),
                        // )
                        .push(label_txt(
                            self.purchase_num.to_string(),
                            PORTION_1,
                            None,
                            None,
                        ))
                        // .push(
                        //     Button::new(&mut self.add_num_button_state, plus_icon())
                        //         .style(style::ActionButton)
                        //         .on_press(ShoppingCartMessage::AddNum(self.id))
                        //         .width(Length::FillPortion(PORTION_1)),
                        // )
                        .width(Length::Fill),
                )
                .width(Length::FillPortion(ShoppingCartComponent::AMOUNT_PORTION)),
            );

        let opbuttons = match self.status {
            CartProdStatus::Editing => {
                let id = self.id;
                row = row.push(
                    TextInput::new(
                        &mut self.timeout_dt_input_state,
                        "2021-12-21 09:00:09",
                        self.yuyue_txt.as_str(),
                        move |input_txt| ShoppingCartMessage::EditInputTimeoutDt(id, input_txt),
                    )
                    .on_submit(ShoppingCartMessage::EditFinish(id))
                    .width(Length::FillPortion(ShoppingCartComponent::TIMEOUT_PORTION))
                    .padding(3),
                );
                vec![
                    op_button(
                        &mut self.immediately_button_state,
                        "确定",
                        Length::FillPortion(PORTION_1),
                        ShoppingCartMessage::EditFinish(id),
                    ),
                    Space::with_width(Length::FillPortion(PORTION_1)).into(),
                    op_button(
                        &mut self.delete_button_state,
                        "取消",
                        Length::FillPortion(PORTION_1),
                        ShoppingCartMessage::EditCancel(id),
                    ),
                ]
            }
            _ => {
                row = row.push(
                    Row::new()
                        .push(label_txt(
                            match self.yuyue_dt {
                                Some(dt) => datetime_fmt(dt),
                                None => "".to_string(),
                            },
                            3,
                            None,
                            None,
                        ))
                        .push(
                            Button::new(&mut self.timeout_button_state, time_clock_icon())
                                .on_press(ShoppingCartMessage::Editing(self.id))
                                .width(Length::FillPortion(1))
                                .style(style::ActionButton),
                        )
                        .width(Length::FillPortion(ShoppingCartComponent::TIMEOUT_PORTION)),
                );
                if !is_selecting {
                    vec![
                        op_button(
                            &mut self.immediately_button_state,
                            "立即购买",
                            Length::FillPortion(1),
                            ShoppingCartMessage::ImmediatelyPurchase(self.id),
                        ),
                        op_button(
                            &mut self.delete_button_state,
                            "删除",
                            Length::FillPortion(1),
                            ShoppingCartMessage::DeleteOneState(self.id),
                        ),
                    ]
                } else {
                    vec![]
                }
            }
        };

        for opb in opbuttons.into_iter() {
            op_box_row = op_box_row.push(opb);
        }
        op_box_row = op_box_row.push(Space::with_width(Length::Units(10)));

        Container::new(
            row.push(
                Container::new(op_box_row)
                    .width(Length::FillPortion(ShoppingCartComponent::OP_PORTION)),
            ),
        )
        .width(Length::Fill)
        .into()
    }
}
