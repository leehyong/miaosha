use iced::{button, container, rule, Background, Color, Vector, scrollable};
use iced::scrollable::{Scrollbar, Scroller};

pub enum Button {
    Primary,
    Secondary,
}

impl button::StyleSheet for Button {
    fn active(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(match self {
                Button::Primary => Color::from_rgb(1.0, 0.0, 0.0),
                Button::Secondary => Color::WHITE,
            })),
            shadow_offset: Vector::new(0.0, 0.0),
            text_color: match self {
                Button::Primary => Color::WHITE,
                Button::Secondary => Color::BLACK,
            },
            border_width: 1.0,
            border_color: match self {
                Button::Primary => Color::from_rgb(1.0, 0.0, 0.0),
                Button::Secondary => Color::WHITE,
            },
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        let active = self.active();

        button::Style {
            background: Some(Background::Color(match self {
                Button::Primary => Color::from_rgb(1.0, 0.0, 0.0),
                Button::Secondary => Color::from_rgb(0.0, 0.75, 1.0),
            })),
            border_color: match self {
                Button::Primary => Color::from_rgb(1.0, 0.0, 0.0),
                Button::Secondary => Color::from_rgb(0.0, 0.75, 1.0),
            },
            shadow_offset: active.shadow_offset + Vector::new(0.0, 1.0),
            ..active
        }
    }
}

const BORDER_COLOR: Color = Color::from_rgb(0.992, 0.6, 0.42);
const HEADER_BACKGROUND_COLOR: Color = Color::from_rgb(0.24, 0.80, 0.60);
const PRIMARY_ROW_COLOR: Color = Color::from_rgb(220.0 / 255.0, 220.0 / 255.0, 220.0 / 255.0);
const SECOND_ROW_COLOR: Color = Color::from_rgb(248.0 / 255.0, 248.0 / 255.0, 1.0);
const ACTION_BUTTON_COLOR: Color = Color::from_rgb(245.0 / 255.0, 1.0, 1.0);

pub struct AppBackgroundColor;

impl container::StyleSheet for AppBackgroundColor {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Background::Color(BORDER_COLOR)),
            ..container::Style::default()
        }
    }
}

pub struct TableHeaderStyle;

impl container::StyleSheet for TableHeaderStyle {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Background::Color(HEADER_BACKGROUND_COLOR)),
            border_width: 1.0,
            border_color: Color::BLACK,
            ..container::Style::default()
        }
    }
}

#[derive(Copy, Clone)]
pub enum TableBodyContentRowStyle {
    Primary,
    Second,
}

impl container::StyleSheet for TableBodyContentRowStyle {
    fn style(&self) -> container::Style {
        let color = match *self {
            TableBodyContentRowStyle::Primary => PRIMARY_ROW_COLOR,
            _ => SECOND_ROW_COLOR,
        };
        container::Style {
            background: Some(Background::Color(color)),
            border_width: 1.0,
            border_color: color,
            ..container::Style::default()
        }
    }
}

pub struct ActionButton;

impl button::StyleSheet for ActionButton {
    fn active(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(Color::from_rgb8(240 ,255, 255))),
            shadow_offset: Vector::new(1.5, 0.5),
            text_color: Color::from_rgb8(0 ,191 ,255),
            border_width: 1.0,
            border_radius:2.0,
            border_color: Color::from_rgb8(240 ,255, 255),
            ..button::Style::default()
        }
    }
}

pub struct ScrollableBarStyle;

impl scrollable::StyleSheet for ScrollableBarStyle {
    fn active(&self) -> Scrollbar {
        let scroller = Scroller{
            color: Color::from_rgb(0.0, 0.75, 1.0),
             border_radius: 2.0,
             border_width: 1.0,
             border_color: Color::from_rgb(0.0, 0.75, 1.0)
        };
        let color = Color::from_rgb8(169 ,169 ,169	);
        Scrollbar{
            scroller,
            background:Some(Background::Color(color)),
            border_radius:0.0,
            border_width:0.0,
            border_color:color
        }
    }

    fn hovered(&self) -> Scrollbar {
        self.active()
    }
}



pub struct AddSubButton;

impl button::StyleSheet for AddSubButton {

    fn active(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(Color::WHITE)),
            shadow_offset: Vector::new(0.0, 0.0),
            text_color: Color::BLACK,
            border_width: 2.0,
            border_radius:3.0,
            border_color: Color::from_rgb8(240 ,255, 255),
            ..button::Style::default()
        }
    }
}

pub struct  UserLineRule;

impl rule::StyleSheet for UserLineRule{
    fn style(&self) -> rule::Style {
        rule::Style {
            fill_mode: rule::FillMode::Percent(100.0),
            color: [0.6, 0.6, 0.6, 0.51].into(),
            width: 1,
            radius: 0.0,
        }
    }
}

pub struct  RowSplitLineRule;

impl rule::StyleSheet for RowSplitLineRule{
    fn style(&self) -> rule::Style {
        rule::Style {
            fill_mode: rule::FillMode::AsymmetricPadding(0, 10),
            color: Color::from_rgb8(0 ,191 ,255),
            width: 2,
            radius: 0.0,
        }
    }
}




pub enum  CheckBoxButton{
    Checked,
    UnChecked,
}

impl  CheckBoxButton{
    const CHECKED_COLOR:Color = Color::from_rgb(1.0 ,0.0, 0.0);
    const UNCHECK_COLOR:Color = Color::WHITE;

}


impl button::StyleSheet for CheckBoxButton {
    fn active(&self) -> button::Style {

        button::Style {
            background: Some(Background::Color(match self{
                Self::Checked => Self::CHECKED_COLOR,
                Self::UnChecked => Self::UNCHECK_COLOR
            })),
            shadow_offset: Vector::new(0.0, 0.0),
            text_color: Self::CHECKED_COLOR,
            border_width: 2.0,
            border_radius:0.0,
            border_color: match self{
                Self::Checked => Self::CHECKED_COLOR,
                Self::UnChecked => Color::BLACK
            },
            ..button::Style::default()
        }
    }
}