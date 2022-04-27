use iced::*;
use log::{debug, error, info, warn};
use serde_json::{json, to_string};

use super::super::*;
use super::style;
use super::PORTION_1;
use crate::models::{AddressInfo, UserInfo, UserState};
use crate::services::delivery_address::DeliveryAddressService;
use crate::utils::*;
use crate::{PKDate, PKDateTime, PKLocal};
use iced::pane_grid::Line;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap, LinkedList};
use std::ops::{Deref, Sub};

#[derive(Default)]
pub struct DeliveryAddressComponent {
    search_input_txt: String,
    search_input_state: text_input::State,
    search_button_state: button::State,
    // search_reset_button_state: button::State,
    scroll_state: scrollable::State,
    // 记录用户的所有收货地址
   pub  user_address: BTreeMap<String, LinkedList<AddressInfo>>,
}

impl DeliveryAddressComponent {
    pub const ACCOUNT_PORTION: u16 = 2;
    pub const RECEIVER_PORTION: u16 = 2;
    pub const AREA_ZONE_PORTION: u16 = 3;
    pub const ADDRESS_PORTION: u16 = 3;
    pub const MOBILE_PHONE_PORTION: u16 = 3;
    pub const EMAIL_PORTION: u16 = 3;
    pub const FIXED_LINE_PHONE_PORTION: u16 = 3;
    pub const OP_PORTION: u16 = 3;

    fn headers_portions() -> (Vec<String>, Vec<u16>) {
        let mut headers = Vec::with_capacity(8);
        for t in [
            "账号",
            "收货人",
            "所在地区",
            "地址",
            "手机",
            "固定电话",
            "电子邮箱",
            "操作",
        ]
        .iter()
        {
            headers.push(t.to_string())
        }
        (
            headers,
            vec![
                Self::ACCOUNT_PORTION,
                Self::RECEIVER_PORTION,
                Self::AREA_ZONE_PORTION,
                Self::ADDRESS_PORTION,
                Self::MOBILE_PHONE_PORTION,
                Self::EMAIL_PORTION,
                Self::FIXED_LINE_PHONE_PORTION,
                Self::OP_PORTION,
            ],
        )
    }

    pub fn update(&mut self, message: DeliveryAddressMessage) -> Command<JdMiaoshaAppMessage> {
        use DeliveryAddressMessage::*;
        // 记录是否点击了选择按钮，避免发送过多的消息
        match message {
            SearchInput(name) => {
                self.search_input_txt = name.trim().to_string();
            }
            Search => {
                // todo 从服务器加载数据
                self.search_input_txt.clear()
            }

            SearchReset => {
                self.search_input_txt.clear();
            }
            LoadFinish(addrs) => {
                if addrs.len() > 0 {
                    let account = addrs.front().unwrap().account.to_owned();
                    self.user_address.insert(account, addrs);
                }
            }
            Setting(account, addrid) =>{
                // let msg = Setting(account.clone(), addrid.clone());
                if let Some(user_addrs) = self.user_address.get(account.as_str()){
                    return Command::batch(
                        user_addrs
                            .iter()
                            .filter(|addr|addr.addr_id == addrid.as_str())
                            .fold(Vec::new(), |mut cmds, (addr)|{
                                let account = addr.account.clone();
                                let ck = addr.cookie.clone();
                                let addr_id = addr.addr_id.clone();
                                cmds.push( Command::perform(async {
                                    DeliveryAddressService::set_order_express_address(addr_id, account, ck).await
                                }, |r|{
                                    match r {
                                        Ok(Some((account, addr_id))) =>{
                                            SetFinish(account, addr_id).into()
                                        }
                                        o @ _ =>{
                                            error!("{:?}", o);
                                            SetFail.into()
                                        }
                                    }
                                }));
                                cmds
                            })
                        );
                }
            }
            SetFinish(account, addrid)=>{
                if let Some(user_addrs) = self.user_address.get_mut(account.as_str()){
                    user_addrs
                        .iter_mut()
                        .map(|addr|{
                            if addr.addr_id == addrid.as_str(){
                                addr.is_latest_receive_addr = true;
                            }else{
                                addr.is_latest_receive_addr = false;
                            }
                        }).count();
                }
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
            .user_address
            .iter_mut()
            .fold(Column::new().spacing(3), |column, (account, addrs)| {
                let total_addr_cnt = addrs.len();
                let one_user_row = Container::new(addrs.iter_mut().enumerate().fold(
                    Column::new().spacing(3),
                    |_column, (idx, addr)| {
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
                    "账号",
                    self.search_input_txt.as_str(),
                    |s| DeliveryAddressMessage::SearchInput(s).into(),
                )
                .size(24)
                .padding(3)
                .width(Length::Units(120))
                .on_submit(DeliveryAddressMessage::Search.into()),
            )
            .push(
                Button::new(
                    &mut self.search_button_state,
                    Text::new("搜索").horizontal_alignment(HorizontalAlignment::Center),
                )
                .style(style::ActionButton)
                .on_press(DeliveryAddressMessage::Search.into()),
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
                .push(scroll), // .push(Rule::horizontal(6).style(style::UserLineRule))
        )
        .width(Length::Fill);
    }
}

const TXT_COLOR: Color = Color::from_rgb(0.0, 191.0 / 255.0, 1.0);

fn label_txt(
    txt: &str,
    portion: u16,
    align_x: Option<HorizontalAlignment>,
    color:Option<Color>
) -> Element<DeliveryAddressMessage> {
    Text::new(txt)
        .horizontal_alignment(align_x.unwrap_or(HorizontalAlignment::Center))
        .vertical_alignment(VerticalAlignment::Center)
        .color(color.unwrap_or(TXT_COLOR))
        .width(Length::FillPortion(portion))
        .into()
}

impl AddressInfo {
    fn new() -> Self {
        Default::default()
    }

    fn update(&mut self, msg: &DeliveryAddressMessage) -> Command<JdMiaoshaAppMessage>{
        use DeliveryAddressMessage::*;
        match msg {
            Setting(s, addr_id) => {
                // todo 调用设置收货地址接口
                self.is_latest_receive_addr = true;
            }
            _ => {}
        }
        Command::none()
    }

    fn view(&mut self, idx: usize, total: usize) -> Element<DeliveryAddressMessage> {
        let mut row = Row::new().align_items(Align::Center);
        let mut op_box_row = Row::new()
            .push(Space::with_width(Length::Units(3)))
            .spacing(3)
            .align_items(Align::Center)
            .width(Length::Fill);
        let op_button = |state,
                         button_name,
                         len: Length,
                         msg: DeliveryAddressMessage|
         -> Element<DeliveryAddressMessage> {
            Button::new(
                state,
                Text::new(button_name).horizontal_alignment(HorizontalAlignment::Center),
            )
            .on_press(msg.into())
            .width(len)
            .style(style::ActionButton)
            .into()
        };

        if self.is_latest_receive_addr {
            op_box_row = op_box_row
                .push(Space::with_width(Length::FillPortion(1)))
                .push(img::right_image())
                .push(Space::with_width(Length::FillPortion(1)))
        }else{
            op_box_row = op_box_row
                .push(op_button(
                    &mut self.set_addr_button_state,
                    "设置为收货地址",
                    Length::FillPortion(4),
                    DeliveryAddressMessage::Setting(self.account.to_string(), self.addr_id.to_string()),
                ))
                .push(Space::with_width(Length::FillPortion(1)));
        }
        // 是否是偶数行
        let is_mid_row = idx == (total >> 1);
        if is_mid_row{
            row = row.push(label_txt(self.account.as_str(),
                                     DeliveryAddressComponent::ACCOUNT_PORTION,
                                     Some(HorizontalAlignment::Center),
                                     Some(Color::from_rgb(1.0, 0.0, 0.0))
            ));
        }else{
            row = row.push(Space::with_width(Length::FillPortion(DeliveryAddressComponent::ACCOUNT_PORTION)));
        }

        row = row
            .push(label_txt(
                &self.receiver,
                DeliveryAddressComponent::RECEIVER_PORTION,
                None,
                None,
            ))
            .push(label_txt(
                &self.area_zone,
                DeliveryAddressComponent::AREA_ZONE_PORTION,
                None,
                None,
            ))
            .push(label_txt(
                &self.address,
                DeliveryAddressComponent::ADDRESS_PORTION,
                None,
                None,
            ))
            .push(label_txt(
                &self.mobile_phone,
                DeliveryAddressComponent::MOBILE_PHONE_PORTION,
                None,
                None,
            ))
            .push(label_txt(
                &self.fixed_line_phone,
                DeliveryAddressComponent::FIXED_LINE_PHONE_PORTION,
                None,
                None,
            ))
            .push(label_txt(
                &self.email,
                DeliveryAddressComponent::EMAIL_PORTION,
                None,
                None,
            ))
            .push(
                Container::new(op_box_row)
                    .width(Length::FillPortion(DeliveryAddressComponent::OP_PORTION)),
            );
         if idx + 1 == total{
            // 最后一行再加上一条水平线
            return Container::new(Column::new()
                .push(row)
                .push(Rule::horizontal(6).style(style::RowSplitLineRule)))
                .width(Length::Fill).into();
        }else {
            Container::new(row).width(Length::Fill).into()
        }
    }
}
