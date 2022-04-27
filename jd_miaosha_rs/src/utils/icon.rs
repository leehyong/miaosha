use iced::*;
const ICONS: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("../../assets/font/iconfont.ttf"),
};

pub fn icon(unicode: char) -> Text {
    Text::new(&unicode.to_string())
        .font(ICONS)
        .width(Length::Units(20))
        .horizontal_alignment(HorizontalAlignment::Center)
        .vertical_alignment(VerticalAlignment::Center)
        .size(20)
}

pub fn time_clock_icon() -> Text{
    icon('\u{e608}')
}

pub fn plus_icon() -> Text{
    icon('\u{e61c}')
}

pub fn sub_icon() -> Text{
    icon('\u{e741}')
}

pub fn right_icon() -> Text{
    icon('\u{e624}')
}

pub fn wrong_icon() -> Text{
    icon('\u{e629}')
}

pub fn loading_icon() -> Text{
    icon('\u{e688}')
}