use iced::*;
use log::{debug, error, info, warn};
use serde_json::{json, to_string};

use super::super::*;
use super::style;
use super::PORTION_1;
use crate::models::{OrderInfo, ProdPlatform};
use crate::services::order::{OrderService, QueryCondition};
use crate::utils::*;
use crate::{PKDate, PKDateTime, PKLocal};
use iced::pane_grid::Line;
use std::collections::{BTreeMap};

pub struct OrderComponent {
    pub search_input_txt: String,
    search_input_state: text_input::State,
    search_button_state: button::State,
    search_reset_button_state: button::State,

    scroll_state: scrollable::State,
    // 记录用户的所有收货地址
    pub orders: BTreeMap<String, BTreeMap<String, OrderInfo>>,
}

impl Default for OrderComponent {
    fn default() -> Self {
        Self {
            search_input_txt: String::default(),
            search_input_state: text_input::State::default(),
            search_button_state: button::State::default(),
            search_reset_button_state: button::State::default(),
            scroll_state: scrollable::State::default(),
            orders: BTreeMap::default(),
        }
    }
}

impl OrderComponent {
    pub const ACCOUNT_PORTION: u16 = 1;
    pub const ORDER_NO_PORTION: u16 = 1;
    pub const NAME_PORTION: u16 = 2;
    pub const PURCHASE_PORTION: u16 = 1;
    pub const TOTAL_PRICE_PORTION: u16 = 1;
    pub const STATUS_PORTION: u16 = 1;
    pub const RECEIVER_PORTION: u16 = 1;

    fn headers_portions() -> (Vec<String>, Vec<u16>) {
        let mut headers = Vec::with_capacity(8);
        for t in [
            "账号",
            "订单号",
            "商品名称",
            "数量",
            "总金额",
            "状态",
            "收货人",
        ]
        .iter()
        {
            headers.push(t.to_string())
        }
        (
            headers,
            vec![
                OrderComponent::ACCOUNT_PORTION,
                OrderComponent::ORDER_NO_PORTION,
                OrderComponent::NAME_PORTION,
                OrderComponent::PURCHASE_PORTION,
                OrderComponent::TOTAL_PRICE_PORTION,
                OrderComponent::STATUS_PORTION,
                OrderComponent::RECEIVER_PORTION,
            ],
        )
    }
    fn update_orders(&mut self, mut orders: BTreeMap<String, OrderInfo>, is_clear:bool){
        if orders.len() > 0 {
            let mut account = "".to_string();
            for v in orders.values() {
                account = v.account.to_string();
                break;
            }
            if let Some(user_orders) = self.orders.get_mut(account.as_str()) {
                if is_clear{
                    user_orders.clear();
                }
                user_orders.append(&mut orders);
            } else {
                self.orders.insert(account, orders);
            }
        }else if is_clear{
            self.orders.clear();
        }
    }

    pub fn update(&mut self, message: OrderMessage) -> Command<JdMiaoshaAppMessage> {
        use OrderMessage::*;
        // 记录是否点击了选择按钮，避免发送过多的消息
        match message {
            SearchInput(input) => {
                self.search_input_txt = input.trim().to_string();
            }
            LoadFinish(mut orders) => {
                self.update_orders(orders, false);
            }
             SearchFinish(mut orders) =>{
                 self.update_orders(orders, true);
             }
            Reset =>{
                self.search_input_txt.clear();
                self.orders.clear();
                return Command::perform(async{}, |_|{
                    JdMiaoshaAppMessage::GlobalOrdersPressed
                });
            }
            _ => {
                info!("{:?}", message);
            }
        }
        return Command::none();
    }

    pub fn view<'a>(&'a mut self) -> Container<'a, JdMiaoshaAppMessage> {
        let (headers, portions) = Self::headers_portions();
        let states: Element<_> = self
            .orders
            .iter_mut()
            .fold(Column::new().spacing(3), |column, (account, orders)| {
                let total_addr_cnt = orders.len();
                let one_user_row = Container::new(orders.iter_mut().enumerate().fold(
                    Column::new().spacing(3),
                    |_column, (idx, (_order_id, addr))| {
                        _column.push(addr.view(idx, total_addr_cnt).map(|msg| msg.into()))
                    },
                ))
                .width(Length::Fill);
                column.push(one_user_row)
            })
            .into();
        let search_row = Row::new()
            .align_items(Align::Center)
            .push(Space::with_width(Length::FillPortion(1)))
            .push(
                TextInput::new(
                    &mut self.search_input_state,
                    "商品名称/商品编号/订单号",
                    self.search_input_txt.as_str(),
                    |s| OrderMessage::SearchInput(s).into(),
                )
                .size(24)
                .padding(3)
                .width(Length::Units(210))
                .on_submit(OrderMessage::Search.into()),
            )
            .push(
                Button::new(
                    &mut self.search_button_state,
                    Text::new("搜索").horizontal_alignment(HorizontalAlignment::Center),
                )
                .style(style::ActionButton)
                .on_press(OrderMessage::Search.into()),
            )
            .push(
                Button::new(
                    &mut self.search_reset_button_state,
                    Text::new("重置").horizontal_alignment(HorizontalAlignment::Center),
                )
                .style(style::ActionButton)
                .on_press(OrderMessage::Reset.into()),
            )
            .spacing(10)
            .padding(3);
        let scroll = Scrollable::new(&mut self.scroll_state)
            .scrollbar_margin(1)
            .scrollbar_width(1)
            .style(style::ScrollableBarStyle)
            .max_height(super::MAX_SCROLL_HEIGHT + 40)
            .push(Container::new(states).width(Length::Fill).center_x());

        return Container::new(
            Column::new()
                .width(Length::Fill)
                .spacing(4)
                .push(search_row)
                .push(Row::with_children(super::get_headers(&headers, &portions)))
                .push(scroll),
        )
        .width(Length::Fill);
    }
}

const TXT_COLOR: Color = Color::from_rgb(0.0, 191.0 / 255.0, 1.0);

fn label_txt<'a, T: AsRef<str>>(
    txt: T,
    portion: u16,
    align_x: Option<HorizontalAlignment>,
    color: Option<Color>,
) -> Element<'a, OrderMessage> {
    Text::new(txt.as_ref())
        .horizontal_alignment(align_x.unwrap_or(HorizontalAlignment::Center))
        .vertical_alignment(VerticalAlignment::Center)
        .color(color.unwrap_or(TXT_COLOR))
        .width(Length::FillPortion(portion))
        .into()
}

impl OrderInfo {
    fn update(&mut self, msg: &OrderMessage) -> Command<JdMiaoshaAppMessage> {
        use OrderMessage::*;
        match msg {
            _ => {
                info!("{:?}", msg);
            }
        }
        Command::none()
    }

    fn view(&mut self, idx: usize, total: usize) -> Element<OrderMessage> {
        let mut row = Row::new().align_items(Align::Center);
        // 是否是偶数行
        let is_mid_row = idx == (total >> 1);
        if is_mid_row{
            row = row.push(label_txt(
                self.account.as_str(),
                OrderComponent::ACCOUNT_PORTION,
                Some(HorizontalAlignment::Center),
                Some(Color::from_rgb(1.0, 0.0, 0.0)),
            ));
        } else {
            row = row.push(Space::with_width(Length::FillPortion(
                OrderComponent::ACCOUNT_PORTION,
            )));
        }
        row = row
            .push(label_txt(
                &self.order_no,
                OrderComponent::ORDER_NO_PORTION,
                None,
                None,
            ))
            .push(label_txt(
                &self.name,
                OrderComponent::NAME_PORTION,
                None,
                None,
            ))
            .push(label_txt(
                self.purchase_num.as_str(),
                OrderComponent::PURCHASE_PORTION,
                None,
                None,
            ))
            .push(label_txt(
                &self.total_price,
                OrderComponent::TOTAL_PRICE_PORTION,
                None,
                None,
            ))
            .push(label_txt(
                &self.status,
                OrderComponent::STATUS_PORTION,
                None,
                None,
            ))
            .push(label_txt(
                &self.receiver,
                OrderComponent::RECEIVER_PORTION,
                None,
                None,
            ));
        // 偶数个地址时， 需要插入一行只有账号名的空白行
        if idx + 1 == total {
            // 最后一行再加上一条水平线
            return Container::new(
                Column::new()
                    .push(row)
                    .push(Rule::horizontal(6).style(style::RowSplitLineRule)),
            )
            .width(Length::Fill)
            .into();
        } else {
            Container::new(row).width(Length::Fill).into()
        }
    }
}
