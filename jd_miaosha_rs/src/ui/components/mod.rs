mod goods;
mod user;
mod delivery_address;
mod shopping_cart;
mod order;

use super::style;
use super::JdMiaoshaAppMessage;
use super::PORTION_1;
pub use goods::GoodsComponent;
pub use user::UserComponent;
pub use shopping_cart::ShoppingCartComponent;
pub use delivery_address::DeliveryAddressComponent;
pub use order::OrderComponent;
use iced::*;

pub const MAX_SCROLL_HEIGHT: u32 = 420;


pub fn get_headers<'a, T:AsRef<str>>(
    headers: &[T],
    portions: &[u16],
) -> Vec<Element<'a, JdMiaoshaAppMessage>> {
    let mut header_elements = Vec::with_capacity(10);
    for (idx, header) in headers.iter().enumerate() {
        header_elements.push(
            Container::new(
                Text::new(header.as_ref())
                    .size(24)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .color(Color::WHITE)
                    .horizontal_alignment(HorizontalAlignment::Center),
            )
            .width(Length::FillPortion(portions[idx]))
            .style(style::TableHeaderStyle)
            .into(),
        )
    }
    header_elements
}
