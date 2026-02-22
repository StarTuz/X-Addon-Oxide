// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

use iced::widget::{button, container, pick_list, text_input};
use iced::{Background, Border, Color, Shadow, Theme};

pub mod palette {
    use iced::Color;

    pub const BACKGROUND: Color = Color::from_rgb(0.12, 0.12, 0.12); // #1e1e1e
    pub const SURFACE: Color = Color::from_rgb(0.18, 0.18, 0.18); // #2d2d2d
    pub const ACCENT_BLUE: Color = Color::from_rgb(0.23, 0.51, 0.96); // #3b82f6
    pub const ACCENT_ORANGE: Color = Color::from_rgb(0.98, 0.45, 0.09); // #f97316
    pub const ACCENT_RED: Color = Color::from_rgb(0.93, 0.25, 0.25); // #ef4444 (Red 500)
    pub const ACCENT_GREEN: Color = Color::from_rgb(0.2, 0.7, 0.3); // #33b34d
    pub const ACCENT_PURPLE: Color = Color::from_rgb(0.66, 0.33, 0.97); // #a855f7 (Electric Violet)
    pub const ACCENT_MAGENTA: Color = Color::from_rgb(0.8, 0.2, 0.8);
    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.9, 0.9, 0.9);
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.6, 0.6, 0.6);
    pub const BORDER: Color = Color::from_rgb(0.25, 0.25, 0.25);
    pub const SURFACE_VARIANT: Color = Color::from_rgb(0.25, 0.25, 0.25);
    pub const ACCENT_CYAN: Color = Color::from_rgb(0.0, 0.8, 0.82); // Cyan 500
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
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

pub fn container_modal(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::SURFACE)),
        border: Border {
            color: palette::BORDER,
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.8),
            offset: iced::Vector::new(0.0, 10.0),
            blur_radius: 30.0,
        },
        ..Default::default()
    }
}

pub fn container_tooltip(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb(0.95, 0.95, 0.95))),
        text_color: Some(Color::BLACK),
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: palette::BORDER,
        },
        ..Default::default()
    }
}

pub fn container_ghost(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.25, 0.25, 0.25, 0.7))),
        border: Border {
            color: palette::ACCENT_BLUE,
            width: 2.0,
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.23, 0.51, 0.96, 0.5),
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 20.0,
        },
        ..Default::default()
    }
}

pub fn container_drop_gap_active(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(palette::ACCENT_BLUE)),
        border: Border {
            radius: 2.0.into(),
            ..Default::default()
        },
        shadow: Shadow {
            color: Color::from_rgba(0.23, 0.51, 0.96, 0.4),
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 8.0,
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
                radius: 6.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            shadow: Shadow {
                color: Color::from_rgba(0.23, 0.51, 0.96, 0.4),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            },
            ..base
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.3, 0.6, 1.0))),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            shadow: Shadow {
                color: Color::from_rgba(0.23, 0.51, 0.96, 0.6),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..base
        },
        _ => button::Style {
            background: Some(Background::Color(palette::ACCENT_BLUE)),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            ..base
        },
    }
}

pub fn button_success(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: None,
        text_color: palette::TEXT_PRIMARY,
        border: Border::default(),
        shadow: Shadow::default(),
    };

    match status {
        button::Status::Active => button::Style {
            background: Some(Background::Color(palette::ACCENT_GREEN)),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            shadow: Shadow {
                color: Color::from_rgba(0.2, 0.7, 0.3, 0.4),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            },
            ..base
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.25, 0.8, 0.35))),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            shadow: Shadow {
                color: Color::from_rgba(0.2, 0.7, 0.3, 0.6),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..base
        },
        _ => button::Style {
            background: Some(Background::Color(palette::ACCENT_GREEN)),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            ..base
        },
    }
}

pub fn button_danger(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: None,
        text_color: palette::TEXT_PRIMARY,
        border: Border::default(),
        shadow: Shadow::default(),
    };

    match status {
        button::Status::Active => button::Style {
            background: Some(Background::Color(palette::ACCENT_RED)),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            shadow: Shadow {
                color: Color::from_rgba(0.93, 0.25, 0.25, 0.4),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            },
            ..base
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(1.0, 0.4, 0.4))),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            shadow: Shadow {
                color: Color::from_rgba(0.93, 0.25, 0.25, 0.6),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..base
        },
        _ => button::Style {
            background: Some(Background::Color(palette::ACCENT_RED)),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            ..base
        },
    }
}

/// Orange action button — used for "Disable All" on conflict photo-streaming groups.
pub fn button_orange(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: None,
        text_color: palette::TEXT_PRIMARY,
        border: Border::default(),
        shadow: Shadow::default(),
    };

    match status {
        button::Status::Active => button::Style {
            background: Some(Background::Color(palette::ACCENT_ORANGE)),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            shadow: Shadow {
                color: Color::from_rgba(0.98, 0.45, 0.09, 0.4),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            },
            ..base
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(1.0, 0.6, 0.2))),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            shadow: Shadow {
                color: Color::from_rgba(0.98, 0.45, 0.09, 0.6),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..base
        },
        _ => button::Style {
            background: Some(Background::Color(palette::ACCENT_ORANGE)),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            ..base
        },
    }
}

pub fn button_danger_glow(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(palette::ACCENT_RED)),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: Color::from_rgba(0.93, 0.25, 0.25, 0.5),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.93, 0.25, 0.25, 0.6),
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 12.0,
        },
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(1.0, 0.4, 0.4))),
            shadow: Shadow {
                color: Color::from_rgba(0.93, 0.25, 0.25, 1.0),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 20.0,
            },
            ..base
        },
        button::Status::Active => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.8, 0.2, 0.2))),
            ..base
        },
        _ => base,
    }
}

pub fn button_primary_glow(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(palette::ACCENT_BLUE)),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: Color::from_rgba(0.23, 0.51, 0.96, 0.5),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.23, 0.51, 0.96, 0.6),
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 12.0,
        },
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.3, 0.6, 1.0))),
            shadow: Shadow {
                color: Color::from_rgba(0.23, 0.51, 0.96, 1.0),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 20.0,
            },
            ..base
        },
        button::Status::Active => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.2, 0.4, 0.8))),
            ..base
        },
        _ => base,
    }
}

pub fn button_toggle_glow(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(palette::ACCENT_PURPLE)),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: Color::from_rgba(0.66, 0.33, 0.97, 0.5),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.66, 0.33, 0.97, 0.6),
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 12.0,
        },
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.75, 0.45, 1.0))),
            shadow: Shadow {
                color: Color::from_rgba(0.66, 0.33, 0.97, 1.0),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 20.0,
            },
            ..base
        },
        button::Status::Active => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.5, 0.2, 0.8))),
            ..base
        },
        _ => base,
    }
}

pub fn button_ai(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: None,
        text_color: palette::TEXT_PRIMARY,
        border: Border::default(),
        shadow: Shadow::default(),
    };

    match status {
        button::Status::Active => button::Style {
            background: Some(Background::Color(palette::ACCENT_PURPLE)),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            shadow: Shadow {
                color: Color::from_rgba(0.66, 0.33, 0.97, 0.5),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 10.0,
            },
            ..base
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.75, 0.45, 1.0))),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            shadow: Shadow {
                color: Color::from_rgba(0.66, 0.33, 0.97, 0.7),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 15.0,
            },
            ..base
        },
        _ => button::Style {
            background: Some(Background::Color(palette::ACCENT_PURPLE)),
            border: Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            text_color: Color::WHITE,
            ..base
        },
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
                radius: 6.0.into(),
            },
            text_color: palette::TEXT_PRIMARY,
            ..base
        },
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.25, 0.25, 0.25))),
            border: Border {
                color: palette::BORDER,
                width: 1.0,
                radius: 6.0.into(),
            },
            text_color: Color::WHITE,
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                offset: iced::Vector::new(0.0, 2.0),
                blur_radius: 8.0,
            },
            ..base
        },
        _ => button::Style {
            background: Some(Background::Color(palette::SURFACE)),
            border: Border {
                color: palette::BORDER,
                width: 1.0,
                radius: 6.0.into(),
            },
            text_color: palette::TEXT_PRIMARY,
            ..base
        },
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
                radius: 6.0.into(),
                ..Default::default()
            },
            ..base
        },
        _ => base,
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

pub fn button_sidebar_inactive(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: None,
        text_color: palette::TEXT_SECONDARY,
        border: Border::default(),
        shadow: Shadow::default(),
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))),
            text_color: palette::TEXT_PRIMARY,
            border: Border {
                radius: 8.0.into(),
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
            radius: 8.0.into(),
        },
        shadow: Shadow::default(),
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.2, 0.2, 0.2))),
            border: Border {
                color: palette::ACCENT_BLUE,
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.23, 0.51, 0.96, 0.2),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..base
        },
        button::Status::Active | button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.2, 0.2, 0.2))),
            border: Border {
                color: palette::ACCENT_BLUE,
                width: 1.2,
                radius: 8.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.23, 0.51, 0.96, 0.4),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 15.0,
            },
            ..base
        },
        _ => base,
    }
}

pub fn button_premium_glow(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(Color::WHITE)),
        text_color: Color::BLACK,
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.5),
        },
        shadow: Shadow {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.6),
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 12.0,
        },
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.9, 0.9, 0.9))),
            shadow: Shadow {
                color: Color::from_rgba(1.0, 1.0, 1.0, 1.0),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 20.0,
            },
            ..base
        },
        button::Status::Active => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.8, 0.8, 0.8))),
            ..base
        },
        _ => base,
    }
}

pub fn button_enable_all(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(palette::ACCENT_BLUE)),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: Color::from_rgba(0.23, 0.51, 0.96, 0.5),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.23, 0.51, 0.96, 0.6),
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 12.0,
        },
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.3, 0.6, 1.0))),
            shadow: Shadow {
                color: Color::from_rgba(0.23, 0.51, 0.96, 1.0),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 20.0,
            },
            ..base
        },
        button::Status::Active => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.2, 0.4, 0.8))),
            ..base
        },
        _ => base,
    }
}
pub fn button_pin_active(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(Color::from_rgba(0.93, 0.25, 0.25, 0.1))),
        text_color: palette::ACCENT_RED,
        border: Border {
            radius: 8.0.into(),
            color: palette::ACCENT_RED,
            width: 1.5,
        },
        shadow: Shadow {
            color: Color::from_rgba(0.93, 0.25, 0.25, 0.4),
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 15.0,
        },
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgba(0.93, 0.25, 0.25, 0.2))),
            shadow: Shadow {
                color: Color::from_rgba(0.93, 0.25, 0.25, 0.7),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 25.0,
            },
            ..base
        },
        button::Status::Active => button::Style {
            background: Some(Background::Color(palette::ACCENT_RED)),
            text_color: Color::WHITE,
            ..base
        },
        _ => base,
    }
}

pub fn button_pin_ghost(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03))),
        text_color: Color::from_rgba(0.93, 0.25, 0.25, 0.4),
        border: Border {
            radius: 8.0.into(),
            color: Color::from_rgba(0.93, 0.25, 0.25, 0.2),
            width: 1.0,
        },
        shadow: Shadow::default(),
    };

    match status {
        button::Status::Hovered | button::Status::Active => button::Style {
            background: Some(Background::Color(Color::from_rgba(0.93, 0.25, 0.25, 0.1))),
            text_color: palette::ACCENT_RED,
            border: Border {
                color: palette::ACCENT_RED,
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.93, 0.25, 0.25, 0.2),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 10.0,
            },
            ..base
        },
        _ => base,
    }
}

pub fn button_ghost_amber(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03))),
        text_color: Color::from_rgba(0.98, 0.65, 0.15, 0.8),
        border: Border {
            radius: 8.0.into(),
            color: Color::from_rgba(0.98, 0.65, 0.15, 0.4),
            width: 1.0,
        },
        shadow: Shadow::default(),
    };

    match status {
        button::Status::Hovered | button::Status::Active => button::Style {
            background: Some(Background::Color(Color::from_rgba(0.98, 0.65, 0.15, 0.15))),
            text_color: Color::from_rgb(0.98, 0.65, 0.15),
            border: Border {
                color: Color::from_rgb(0.98, 0.65, 0.15),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.98, 0.65, 0.15, 0.3),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 10.0,
            },
            ..base
        },
        _ => base,
    }
}

pub fn button_ghost_teal(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03))),
        text_color: Color::from_rgba(0.15, 0.85, 0.70, 0.8),
        border: Border {
            radius: 8.0.into(),
            color: Color::from_rgba(0.15, 0.85, 0.70, 0.4),
            width: 1.0,
        },
        shadow: Shadow::default(),
    };

    match status {
        button::Status::Hovered | button::Status::Active => button::Style {
            background: Some(Background::Color(Color::from_rgba(0.15, 0.85, 0.70, 0.15))),
            text_color: Color::from_rgb(0.15, 0.85, 0.70),
            border: Border {
                color: Color::from_rgb(0.15, 0.85, 0.70),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.15, 0.85, 0.70, 0.3),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 10.0,
            },
            ..base
        },
        _ => base,
    }
}

pub fn button_ghost_indigo(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03))),
        text_color: Color::from_rgba(0.40, 0.35, 0.90, 0.8),
        border: Border {
            radius: 8.0.into(),
            color: Color::from_rgba(0.40, 0.35, 0.90, 0.4),
            width: 1.0,
        },
        shadow: Shadow::default(),
    };

    match status {
        button::Status::Hovered | button::Status::Active => button::Style {
            background: Some(Background::Color(Color::from_rgba(0.40, 0.35, 0.90, 0.15))),
            text_color: Color::from_rgb(0.40, 0.35, 0.90),
            border: Border {
                color: Color::from_rgb(0.40, 0.35, 0.90),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.40, 0.35, 0.90, 0.3),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 10.0,
            },
            ..base
        },
        _ => base,
    }
}

pub fn button_neumorphic(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(palette::SURFACE)),
        text_color: palette::TEXT_PRIMARY,
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: palette::BORDER,
        },
        shadow: Shadow {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.45),
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 15.0,
        },
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.25, 0.25, 0.25))),
            shadow: Shadow {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.2),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 15.0,
            },
            ..base
        },
        button::Status::Active => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.15, 0.15, 0.15))),
            shadow: Shadow::default(), // "Pressed" into the surface
            ..base
        },
        _ => base,
    }
}

pub fn pick_list_primary(_theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let base = pick_list::Style {
        text_color: palette::TEXT_PRIMARY,
        placeholder_color: palette::TEXT_SECONDARY,
        handle_color: palette::TEXT_PRIMARY,
        background: Background::Color(palette::SURFACE),
        border: Border {
            radius: 8.0.into(),
            width: 1.1,
            color: palette::BORDER,
        },
    };

    match status {
        pick_list::Status::Hovered => pick_list::Style {
            border: Border {
                color: palette::ACCENT_BLUE,
                width: 1.1,
                radius: 8.0.into(),
            },
            ..base
        },
        pick_list::Status::Opened => pick_list::Style {
            border: Border {
                color: palette::ACCENT_BLUE,
                width: 1.1,
                radius: 8.0.into(),
            },
            ..base
        },
        _ => base,
    }
}

pub fn text_input_primary(_theme: &Theme, _status: text_input::Status) -> text_input::Style {
    text_input::Style {
        background: Background::Color(palette::SURFACE),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: palette::BORDER,
        },
        icon: Color::WHITE,
        placeholder: palette::TEXT_SECONDARY,
        value: palette::TEXT_PRIMARY,
        selection: palette::ACCENT_BLUE,
    }
}
pub fn button_region_header(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(palette::SURFACE)),
        text_color: palette::TEXT_PRIMARY,
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: palette::BORDER,
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
            offset: iced::Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.22, 0.22, 0.22))),
            border: Border {
                color: palette::ACCENT_BLUE,
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.23, 0.51, 0.96, 0.15),
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(palette::BACKGROUND)),
            shadow: Shadow::default(),
            ..base
        },
        _ => base,
    }
}

pub fn pick_list_secondary(_theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let base = pick_list::Style {
        text_color: palette::TEXT_PRIMARY,
        placeholder_color: palette::TEXT_SECONDARY,
        handle_color: palette::TEXT_PRIMARY,
        background: Background::Color(palette::BACKGROUND),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: palette::BORDER,
        },
    };

    match status {
        pick_list::Status::Hovered => pick_list::Style {
            border: Border {
                color: palette::BORDER,
                width: 1.0,
                radius: 6.0.into(),
            },
            background: Background::Color(Color::from_rgb(0.2, 0.2, 0.2)),
            ..base
        },
        _ => base,
    }
}

pub fn button_success_glow(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(palette::ACCENT_GREEN)),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: Color::from_rgba(0.2, 0.7, 0.3, 0.5),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.2, 0.7, 0.3, 0.6),
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 12.0,
        },
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.25, 0.8, 0.35))),
            shadow: Shadow {
                color: Color::from_rgba(0.2, 0.7, 0.3, 1.0),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 20.0,
            },
            ..base
        },
        button::Status::Active => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.15, 0.6, 0.25))),
            ..base
        },
        _ => base,
    }
}

pub fn button_cyan_glow(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(palette::ACCENT_CYAN)),
        text_color: Color::BLACK,
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: Color::from_rgba(0.0, 0.8, 0.82, 0.5),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.8, 0.82, 0.6),
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 12.0,
        },
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.2, 0.9, 0.92))),
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.8, 0.82, 1.0),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 20.0,
            },
            ..base
        },
        button::Status::Active => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.0, 0.7, 0.72))),
            ..base
        },
        _ => base,
    }
}

pub fn button_orange_glow(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(palette::ACCENT_ORANGE)),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: Color::from_rgba(0.98, 0.45, 0.09, 0.5),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.98, 0.45, 0.09, 0.6),
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 12.0,
        },
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(1.0, 0.55, 0.2))),
            shadow: Shadow {
                color: Color::from_rgba(0.98, 0.45, 0.09, 1.0),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 20.0,
            },
            ..base
        },
        button::Status::Active => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.9, 0.4, 0.05))),
            ..base
        },
        _ => base,
    }
}

pub fn button_purple_glow(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(palette::ACCENT_PURPLE)),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: Color::from_rgba(0.66, 0.33, 0.97, 0.5),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.66, 0.33, 0.97, 0.6),
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 12.0,
        },
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.75, 0.45, 1.0))),
            shadow: Shadow {
                color: Color::from_rgba(0.66, 0.33, 0.97, 1.0),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 20.0,
            },
            ..base
        },
        button::Status::Active => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.6, 0.3, 0.9))),
            ..base
        },
        _ => base,
    }
}

pub fn button_magenta_glow(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(palette::ACCENT_MAGENTA)),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            width: 1.0,
            color: Color::from_rgba(0.8, 0.2, 0.8, 0.5),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.8, 0.2, 0.8, 0.6),
            offset: iced::Vector::new(0.0, 0.0),
            blur_radius: 12.0,
        },
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.9, 0.3, 0.9))),
            shadow: Shadow {
                color: Color::from_rgba(0.8, 0.2, 0.8, 1.0),
                offset: iced::Vector::new(0.0, 0.0),
                blur_radius: 20.0,
            },
            ..base
        },
        button::Status::Active => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.6, 0.1, 0.6))),
            ..base
        },
        _ => base,
    }
}
