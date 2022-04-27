use std::collections::LinkedList;
use std::sync::Arc;

use iced::*;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{json, to_string};

use super::super::*;
use super::style;
use super::PORTION_1;
use crate::models::{
    Area, GoodsState, DEFAULT_ADDR, DEFAULT_ADDR_NAMES, PROVINCES, PROVINCE_NAMES,
};
use crate::services::area::AreaService;
use crate::services::goods::GoodsService;
use crate::services::shopping_cart::ShoppingCartService;
use crate::ui::JdMiaoshaAppMessage::Goods;
use crate::utils::icon::{plus_icon, sub_icon};
use crate::{PRESALE, YUYUE};
use core::fmt::Alignment::Left;
use std::borrow::Cow;
use crate::utils::datetime_fmt_option;

#[derive(Default)]
pub struct GoodsComponent {
    // 商品搜索输入框的内容
    pub search_input_txt: String,
    pub cur_state: Option<JdMiaoshaAppMessage>,
    // 地区选择相关 -- 开始
    level1_pick_state: pick_list::State<String>,
    level2_pick_state: pick_list::State<String>,
    level3_pick_state: pick_list::State<String>,
    level4_pick_state: pick_list::State<String>,
    leve2_pick_area: Area,
    leve3_pick_area: Area,
    leve4_pick_area: Area,
    leve2_pick_list: Vec<String>,
    leve3_pick_list: Vec<String>,
    leve4_pick_list: Vec<String>,
    selected_area_ids: [i64; 4],
    // 地区选择相关 -- 结束
    // 滚动条
    pub scroll_state: scrollable::State,
    // 商品搜索输入框
    pub goods_search_input_state: text_input::State,
    // 商品搜索按钮
    pub goods_search_button_state: button::State,
    pub clear_all_goods_button_state: button::State,
    pub activate_code: String,
    pub goods: LinkedList<GoodsState>,
}

impl GoodsComponent {
    pub const HEADERS: [&'static str; 7] = [
        "商品名称",
        "商品SKU",
        "原价",
        "现价",
        "库存",
        "数量",
        // "限购",
        "操作",
    ];
    pub const NAME_PORTION: u16 = 3;
    pub const SKU_PORTION: u16 = 2;
    pub const ORI_PRICE_PORTION: u16 = 1;
    pub const CUR_PRICE_PORTION: u16 = 1;
    pub const STATUS_PORTION: u16 = 1;
    pub const PURCHASE_NUM_PORTION: u16 = 2;
    // pub const LIMIT_PURCHASE_PORTION: u16 = 1;
    pub const OP_PORTION: u16 = 2;

    pub const PORTIONS: [u16; Self::HEADERS.len()] = [
        Self::NAME_PORTION,
        Self::SKU_PORTION,
        Self::ORI_PRICE_PORTION,
        Self::CUR_PRICE_PORTION,
        Self::STATUS_PORTION,
        Self::PURCHASE_NUM_PORTION,
        // Self::LIMIT_PURCHASE_PORTION,
        Self::OP_PORTION,
    ];

    pub fn txt_total_portion() -> u16 {
        Self::PORTIONS.iter().sum::<u16>() - Self::PORTIONS[Self::PORTIONS.len() - 1]
    }

    pub fn get_addr_str(&self) -> String {
        if self.selected_area_ids[2] != 0 {
            format!(
                "{}_{}_{}_{}",
                self.selected_area_ids[0],
                self.selected_area_ids[1],
                self.selected_area_ids[2],
                self.selected_area_ids[3],
            )
        } else {
            DEFAULT_ADDR.to_owned() // 北京(1), 朝阳区(72), 四环到五环之间(2839)
        }
    }

    pub fn update(&mut self, message: GoodsMessage) -> Command<JdMiaoshaAppMessage> {
        use GoodsMessage::*;
        let code = self.activate_code.clone();
        let mut update = |msg, sku1: &str| {
            Command::batch(
                self.goods
                    .iter_mut()
                    .filter(|gs| gs.sku == sku1)
                    .map(|gs| gs.update(code.clone(), msg)),
            )
        };
        match message {
            AddToShoppingCart(ref sku)
            | AddToShoppingCartByPresale(ref sku, _, _)
            | ImmediatelyPurchase(ref sku)
            | TimeoutPurchase(ref sku) => {
                if code.len() > 0 {
                    let msg = message.clone();
                    info!("{:?}", &msg);
                    return update(&msg, sku.as_str());
                }
            }

            AddNum(sku) => {
                let sku1 = sku.clone();
                update(&AddNum(sku1), sku.as_str());
            }
            SubNum(sku) => {
                let sku1 = sku.clone();
                update(&SubNum(sku1), sku.as_str());
            }

            SearchInput(s) => {
                self.search_input_txt = s.clone();
            }
            Search => {
                self.cur_state = Some(GoodsMessage::Search.into());
                if self.search_input_txt.len() > 0 {
                    let search_txt = self.search_input_txt.to_string();
                    let area_id = self.get_addr_str();
                    return Command::perform(
                        async move {
                            GoodsService::get_prod_info(search_txt.as_str(), area_id.as_str()).await
                        },
                        |t| match t {
                            Ok(gs) => GoodsMessage::SearchFinish(Some(gs)).into(),
                            Err(e) => {
                                error!("get_prod_info {:?}", e);
                                GoodsMessage::SearchFinish(None).into()
                            }
                        },
                    );
                }
            }
            SearchFinish(d) => {
                if let Some(data) = d {
                    if self
                        .goods
                        .iter()
                        .find(|gs| gs.sku == self.search_input_txt.as_str())
                        .is_none()
                    {
                        // 避免重复加入相同 sku 的商品
                        if self.goods.len() > 50 {
                            // 最大支持的商品数， 多了待优化.
                            self.goods.pop_back();
                        }
                        // let need_query_purchase_link = data.purchase_url.is_empty();
                        // let sku = data.sku.clone();
                        // 保证新搜索的商品排在最前面显示
                        self.goods.push_front(data);
                        // 搜索完了后， 清空已输入的商品 SKU
                        self.search_input_txt.clear();
                        // if need_query_purchase_link {
                        //     info!("Searching purchase url of {}", sku.as_str());
                        //     return Command::perform(
                        //         async move { GoodsService::get_goods_seckill_link(sku).await },
                        //         |r|
                        //             match r{
                        //                 Ok((sku, url)) => GoodsMessage::QueryPurchaseLinkFinish(Some((sku, url))).into(),
                        //                 Err(e) =>{
                        //                     error!("Searching purchase url error {:?}", e);
                        //                     GoodsMessage::QueryPurchaseLinkFinish(None).into()
                        //                 }
                        //             }
                        //     );
                        // }
                    }
                } else {
                    // 搜索完了后， 清空已输入的商品 SKU
                    self.search_input_txt.clear();
                }
            }
            QueryPurchaseLinkFinish(res) => {
                if let Some((sku, url)) = res {
                    if !url.is_empty() {
                        if let Some(p) = self.goods.iter_mut().find(|gs| gs.sku == sku.as_str()) {
                            p.purchase_url = url.clone();
                            return Command::perform(async move { (sku, url) }, |(sku, url)| {
                                MaybeUpdatePurchaseLink(sku, url).into()
                            });
                        }
                    }
                }
            }

            ClearAll => {
                self.goods.clear();
            }

            MonitorStock(cart_goods_id, sku) => {
                // 商品的配送地址
                let addr = self.get_addr_str();
                return Command::perform(
                    async move { GoodsService::get_prod_stock(cart_goods_id, sku, addr).await },
                    |r| {
                        match r {
                            Ok((cart_goods_id, stock_status)) => {
                                if stock_status.is_stock() {
                                    // 有库存之后才去更新商品库存， 同时下单购买
                                    return ShoppingCartMessage::UpdateStock(
                                        cart_goods_id,
                                        stock_status,
                                    )
                                    .into();
                                }
                            }
                            Err(e) => {
                                error!("{:?}", e);
                            }
                        }
                        JdMiaoshaAppMessage::GlobalNoop
                    },
                );
            }

            _ => {
                info!("{:?}", message);
            }
        }
        Command::none()
    }

    fn area_list(area: &Area) -> Vec<String> {
        area.values().map(|s| s.to_owned()).collect::<Vec<String>>()
    }

    pub fn update_area(&mut self, message: AreaMessage) -> Command<JdMiaoshaAppMessage> {
        use AreaMessage::*;
        let get_id_by_name = |iters: &Area, name: &str| {
            let ids: Vec<i64> = iters
                .iter()
                .filter(|item| item.1 == name)
                .map(|item| item.0.clone())
                .collect();
            if ids.len() > 0 {
                Some(ids[0])
            } else {
                None
            }
        };
        match message {
            LevelChanged(level, ref area_name) => {
                if let Some(id) = get_id_by_name(
                    match level {
                        0 => &*PROVINCES,
                        1 => &self.leve2_pick_area,
                        2 => &self.leve3_pick_area,
                        3 => &self.leve4_pick_area,
                        _ => {
                            unreachable!()
                        }
                    },
                    area_name.as_str(),
                ) {
                    self.selected_area_ids[level as usize] = id;
                    for idx in (level + 1) as usize..self.selected_area_ids.len() {
                        self.selected_area_ids[idx] = 0;
                    }
                    // 清空后面的选择
                    if level <= 2 {
                        self.leve4_pick_area.clear();
                        self.leve4_pick_list.clear();
                        if level <= 1 {
                            self.leve3_pick_area.clear();
                            self.leve3_pick_list.clear();
                            if level == 0 {
                                self.leve2_pick_area.clear();
                                self.leve2_pick_list.clear();
                            }
                        }
                    }
                    return Command::perform(AreaService::get_sub_areas(id), move |t| match t {
                        Ok(v) => AreaMessage::LevelChangedFinished(level, v).into(),
                        Err(_) => AreaMessage::LevelChangedFinished(level, Area::new()).into(),
                    });
                }
            }

            LevelChangedFinished(level, v) => {
                // 地址变更后，不再有新的子地址时，说明已经选到最后的地址了
                // 此时需要对地址进行保存
                let is_save = v.is_empty();
                if level == 0 {
                    self.leve2_pick_area = v;
                    self.leve2_pick_list = Self::area_list(&self.leve2_pick_area);
                } else if level == 1 {
                    self.leve3_pick_area = v;
                    self.leve3_pick_list = Self::area_list(&self.leve3_pick_area);
                } else if level == 2 {
                    self.leve4_pick_area = v;
                    self.leve4_pick_list = Self::area_list(&self.leve4_pick_area);
                }
                if is_save {
                    // 保存地址
                    let addr = self.get_addr_str();
                    info!("addr:{}", addr);
                    return Command::perform(
                        async move { AreaService::store_area(addr).await },
                        |t| {
                            if let Err(e) = t {
                                error!("{:?}", e);
                            }
                            AreaStoreFinished.into()
                        },
                    );
                }
            }
            AreaStoreFinished => {
                info!("Area StoreFinished");
            }
            AreaLoad(addrs) => {
                if addrs.is_empty() {
                    return Command::none();
                }
                info!("addrs:{}", addrs);
                let addrs = addrs
                    .split('_')
                    .map(|u| u.parse::<i64>().unwrap_or(0))
                    .collect::<Vec<i64>>();
                for (idx, id) in addrs.iter().enumerate() {
                    self.selected_area_ids[idx] = id.clone();
                }
                return Command::perform(
                    async move {
                        AreaService::get_many_sub_areas(vec![addrs[0], addrs[1], addrs[2]]).await
                    },
                    |d| AreaLoadFinished(d).into(),
                );
            }
            AreaLoadFinished(mut d) => {
                info!("AreaLoadFinished:{}", d.len());
                self.leve4_pick_area = d.pop().unwrap_or_default();
                self.leve3_pick_area = d.pop().unwrap_or_default();
                self.leve2_pick_area = d.pop().unwrap_or_default();
                self.leve2_pick_list = Self::area_list(&self.leve2_pick_area);
                self.leve3_pick_list = Self::area_list(&self.leve3_pick_area);
                self.leve4_pick_list = Self::area_list(&self.leve4_pick_area);
            }
        }
        Command::none()
    }

    pub fn view(&mut self) -> Container<JdMiaoshaAppMessage> {
        let mut rows = vec![];
        let mut area_row = Row::new()
            .padding(3)
            .spacing(3)
            .push(
                Text::new("配送地区:")
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .vertical_alignment(VerticalAlignment::Center),
            )
            .push(Space::with_width(Length::Units(4)));

        //
        let mut areas_settings = vec![
            (
                &mut self.level1_pick_state,
                &*PROVINCES,
                &*PROVINCE_NAMES,
                0,
            ),
            (
                &mut self.level2_pick_state,
                &self.leve2_pick_area,
                &self.leve2_pick_list,
                1,
            ),
            (
                &mut self.level3_pick_state,
                &self.leve3_pick_area,
                &self.leve3_pick_list,
                2,
            ),
            (
                &mut self.level4_pick_state,
                &self.leve4_pick_area,
                &self.leve4_pick_list,
                3,
            ),
        ];
        for (st, maps, names, level) in areas_settings {
            if names.len() > 0 {
                area_row = area_row.push(PickList::new(
                    st,
                    names,
                    Some(
                        maps.get(&self.selected_area_ids[level])
                            .unwrap_or(&"".to_string())
                            .to_string(),
                    ),
                    move |t| AreaMessage::LevelChanged(level as u8, t).into(),
                ));
            }
        }

        rows.push(
            Column::with_children(vec![
                Row::new()
                    .push(
                        Column::new()
                            .align_items(Align::Start)
                            .width(Length::FillPortion(3))
                            .push(area_row),
                    )
                    .push(
                        Column::new().width(Length::FillPortion(1)).push(
                            Row::new()
                                .push(
                                    Container::new(
                                        TextInput::new(
                                            &mut self.goods_search_input_state,
                                            "商品SKU",
                                            self.search_input_txt.as_str(),
                                            |s| GoodsMessage::SearchInput(s).into(),
                                        )
                                        .width(Length::Fill)
                                        .size(24)
                                        .padding(3)
                                        .on_submit(GoodsMessage::Search.into()),
                                    )
                                    .padding(3)
                                    .width(Length::FillPortion(3)),
                                )
                                .push(
                                    Button::new(
                                        &mut self.goods_search_button_state,
                                        Text::new("搜索")
                                            .width(Length::Fill)
                                            .horizontal_alignment(HorizontalAlignment::Center),
                                    )
                                    .width(Length::FillPortion(1))
                                    .style(style::ActionButton)
                                    .on_press(GoodsMessage::Search.into()),
                                )
                                .spacing(10)
                                .align_items(Align::Center),
                        ),
                    )
                    .into(),
                Row::with_children(super::get_headers(&Self::HEADERS, &Self::PORTIONS))
                    .width(Length::Fill)
                    .into(),
            ])
            .into(),
        );

        let goods: Element<_> = self
            .goods
            .iter_mut()
            .enumerate()
            .fold(Column::new(), |column, (idx, goods_state)| {
                column.push(goods_state.view(idx).map(move |msg| msg.into()))
            })
            .into();
        rows.push(
            Scrollable::new(&mut self.scroll_state)
                .scrollbar_margin(1)
                .scrollbar_width(1)
                .style(style::ScrollableBarStyle)
                .push(Container::new(goods).width(Length::Fill).center_x())
                .max_height(super::MAX_SCROLL_HEIGHT)
                .into(),
        );
        rows.push(
            Column::new()
                .width(Length::Fill)
                .push(
                    Button::new(
                        &mut self.clear_all_goods_button_state,
                        Text::new("清空所有商品").horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .on_press(GoodsMessage::ClearAll.into())
                    .style(style::ActionButton),
                )
                .align_items(Align::Start)
                .into(),
        );
        Container::new(Column::with_children(rows).spacing(4))
    }
}

impl GoodsState {
    fn new() -> Self {
        Self {
            purchase_num: 1,
            limit_purchase: 3,
            ..Default::default()
        }
    }

    fn add_to_cart(&self, code:String) -> Command<JdMiaoshaAppMessage> {
        let body = json!({
            "sku":&self.sku,
            "name":&self.name,
            "purchase_num":self.purchase_num,
            "yuyue_dt":datetime_fmt_option(&self.yuyue_dt),
            "ori_price":&self.ori_price,
            "cur_price":&self.cur_price,
            "purchase_type":&self.purchase_type,
            "is_stock":self.status.is_stock_u8(),
            "purchase_url":&self.purchase_url,
        })
        .to_string();
        return Command::perform(
            async move { ShoppingCartService::create_cart_goods(code, body).await },
            |r| {
                if let Err(e) = r {
                    error!("{:?}", e);
                }
                ShoppingCartMessage::Loading.into()
            },
        );
    }

    fn update(&mut self, code: String, msg: &GoodsMessage) -> Command<JdMiaoshaAppMessage> {
        use GoodsMessage::*;
        match msg {
            AddToShoppingCart(_) => {
                if self.purchase_type.eq(PRESALE) {
                    let sku = self.sku.to_owned();
                    return Command::perform(
                        async move { GoodsService::get_presale_info(sku).await },
                        |r| match r {
                            Ok((sku, yuyue_dt, href)) => {
                                AddToShoppingCartByPresale(sku, yuyue_dt, href).into()
                            }
                            Err(e) => {
                                error!("获取预售商品的预售时间失败，无法加入购物车, {:?}", e);
                                JdMiaoshaAppMessage::GlobalNoop
                            }
                        },
                    );
                } else {
                    let sku = self.sku.clone();
                    let num = self.purchase_num.clone();
                    return Command::batch(vec![
                        Command::perform(async move {sku}, move|sku| {
                            UserMessage::AddToCartAndLoadCartGoodsSkuUuids(num, Some(sku)).into()
                        }),
                        self.add_to_cart(code)
                    ]);
                }
            }
            AddToShoppingCartByPresale(_, yuyue_dt,href) =>{
                self.yuyue_dt = yuyue_dt.clone();
                self.purchase_url = href.clone();
                return self.add_to_cart(code);
            }
            AddNum(_) => {
                self.purchase_num = std::cmp::min(self.purchase_num + 1, u8::MAX);
                info!("AddNum, {}", self.purchase_num);
            }
            SubNum(_) => {
                self.purchase_num -= 1;
                info!("SubNum, {}", self.purchase_num);
                if self.purchase_num < 1 {
                    self.purchase_num = 1
                }
            }
            _ => {
                info!("{:?}", msg);
            }
        }
        Command::none()
    }

    fn view(&mut self, idx: usize) -> Element<GoodsMessage> {
        let txt_element = |content, portion| -> Element<GoodsMessage> {
            Text::new(content)
                .width(Length::FillPortion(portion))
                .horizontal_alignment(HorizontalAlignment::Center)
                .vertical_alignment(VerticalAlignment::Center)
                .into()
        };
        Row::new()
            .align_items(Align::Center)
            .width(Length::Fill)
            .push(
                Container::new(
                    Row::new()
                        .align_items(Align::Center)
                        .push(txt_element(self.name.clone(), GoodsComponent::NAME_PORTION))
                        .push(txt_element(self.sku.clone(), GoodsComponent::SKU_PORTION))
                        .push(txt_element(
                            self.ori_price.clone(),
                            GoodsComponent::ORI_PRICE_PORTION,
                        ))
                        .push(txt_element(
                            self.cur_price.clone(),
                            GoodsComponent::CUR_PRICE_PORTION,
                        ))
                        .push(txt_element(
                            self.status.to_string(),
                            GoodsComponent::STATUS_PORTION,
                        ))
                        .push(
                            Container::new(
                                Row::new()
                                    .align_items(Align::Center)
                                    .push(
                                        Button::new(&mut self.sub_num_button_state, sub_icon())
                                            .style(style::ActionButton)
                                            .on_press(GoodsMessage::SubNum(self.sku.clone()))
                                            .width(Length::FillPortion(PORTION_1)),
                                    )
                                    .push(txt_element(self.purchase_num.to_string(), 1))
                                    .push(
                                        Button::new(&mut self.add_num_button_state, plus_icon())
                                            .style(style::ActionButton)
                                            .on_press(GoodsMessage::AddNum(self.sku.clone()))
                                            .width(Length::FillPortion(PORTION_1)),
                                    )
                                    .width(Length::Fill),
                            )
                            .width(Length::FillPortion(GoodsComponent::PURCHASE_NUM_PORTION)),
                        )
                        // .push(txt_element(
                        //     self.limit_purchase.to_string(),
                        //     GoodsComponent::LIMIT_PURCHASE_PORTION,
                        // ))
                        .width(Length::Fill),
                )
                .width(Length::FillPortion(GoodsComponent::txt_total_portion()))
                .style(if idx % 2 == 0 {
                    style::TableBodyContentRowStyle::Second
                } else {
                    style::TableBodyContentRowStyle::Primary
                }),
            )
            .push(
                Container::new(
                    Row::new()
                        .padding(1)
                        .spacing(2)
                        .push(
                            Button::new(
                                &mut self.add_to_cart_button_state,
                                Text::new("加入购物车")
                                    .horizontal_alignment(HorizontalAlignment::Center),
                            )
                            // .width(Length::FillPortion(1))
                            .on_press(GoodsMessage::AddToShoppingCart(self.sku.clone()))
                            .style(style::ActionButton),
                        )
                        // .push(
                        //     Button::new(
                        //         &mut self.immediately_buy_button_state,
                        //         Text::new("立即购买")
                        //             .horizontal_alignment(HorizontalAlignment::Center),
                        //     )
                        //     .width(Length::FillPortion(1))
                        //     .on_press(GoodsMessage::ImmediatelyPurchase(self.sku.clone()))
                        //     .style(style::ActionButton),
                        // )
                        // .push(
                        //     Button::new(
                        //         &mut self.timeout_purchase_button_state,
                        //         Text::new("预约购买")
                        //             .horizontal_alignment(HorizontalAlignment::Center),
                        //     )
                        //     .width(Length::FillPortion(1))
                        //     .on_press(GoodsMessage::TimeoutPurchase(self.sku.clone()))
                        //     .style(style::ActionButton),
                        // )
                        // .push(Space::with_width(Length::Units(6))),
                        .push(Space::with_width(Length::FillPortion(PORTION_1))),
                )
                .width(Length::FillPortion(GoodsComponent::OP_PORTION)),
            )
            .into()
    }
}
