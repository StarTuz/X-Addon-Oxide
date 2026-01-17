use iced::widget::{button, container};
use iced::{Background, Border, Color, Shadow, Theme};

pub mod palette {
    use iced::Color;

    pub const BACKGROUND: Color = Color::from_rgb(0.12, 0.12, 0.12); // #1e1e1e
    pub const SURFACE: Color = Color::from_rgb(0.18, 0.18, 0.18); // #2d2d2d
    pub const ACCENT_BLUE: Color = Color::from_rgb(0.23, 0.51, 0.96); // #3b82f6
    pub const ACCENT_ORANGE: Color = Color::from_rgb(0.98, 0.45, 0.09); // #f97316
    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.9, 0.9, 0.9);
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.6, 0.6, 0.6);
    pub const BORDER: Color = Color::from_rgb(0.25, 0.25, 0.25);
}

// Container Styles
pub fn container_sidebar(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::BACKGROUND)),
        border: Border {
            color: palette::BORDER,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn container_main_content(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::BACKGROUND)),
        ..Default::default()
    }
}

pub fn container_card(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::SURFACE)),
        border: Border {
            color: palette::BORDER,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

// Button Styles
pub fn button_primary(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: None,
        text_color: palette::TEXT_PRIMARY,
        border: Border::default(),
        shadow: Shadow::default(),
    };

    match status {
        button::Status::Active => button::Style {
            background: Some(Background::Color(palette::ACCENT_BLUE)),
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            ..base
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.3, 0.6, 1.0))),
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            ..base
        },
        _ => base,
    }
}

pub fn button_secondary(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: None,
        text_color: palette::TEXT_PRIMARY,
        border: Border::default(),
        shadow: Shadow::default(),
    };

    match status {
        button::Status::Active => button::Style {
            background: Some(Background::Color(palette::SURFACE)),
            border: Border {
                color: palette::BORDER,
                width: 1.0,
                radius: 4.0.into(),
            },
            text_color: palette::TEXT_PRIMARY,
            ..base
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.25, 0.25, 0.25))),
            border: Border {
                color: palette::BORDER,
                width: 1.0,
                radius: 4.0.into(),
            },
            text_color: Color::WHITE,
            ..base
        },
        _ => button::Style {
            background: Some(Background::Color(palette::SURFACE)),
            border: Border {
                color: palette::BORDER,
                width: 1.0,
                radius: 4.0.into(),
            },
            text_color: palette::TEXT_PRIMARY,
            ..base
        },
    }
}

pub fn button_sidebar_active(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: palette::TEXT_PRIMARY,
        border: Border::default(),
        shadow: Shadow::default(),
    }
}

pub fn button_sidebar_inactive(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: palette::TEXT_SECONDARY,
        border: Border {
            width: 0.0,
            radius: 0.0.into(),
            ..Default::default()
        },
        shadow: Shadow::default(),
    }
}

pub fn button_ghost(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: None,
        text_color: palette::TEXT_PRIMARY,
        border: Border::default(),
        shadow: Shadow::default(),
    };

    match status {
        button::Status::Hovered | button::Status::Active => button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.1))),
            text_color: Color::WHITE,
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..base
        },
        _ => base,
    }
}

pub fn button_card(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(palette::SURFACE)),
        text_color: palette::TEXT_PRIMARY,
        border: Border {
            color: palette::BORDER,
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
    };

    match status {
        button::Status::Hovered => button::Style {
            border: Border {
                color: palette::ACCENT_BLUE,
                width: 1.0,
                radius: 6.0.into(),
            },
            ..base
        },
        button::Status::Active => button::Style {
            border: Border {
                color: palette::ACCENT_BLUE,
                width: 1.0,
                radius: 6.0.into(),
            },
            ..base
        },
        _ => base,
    }
}
