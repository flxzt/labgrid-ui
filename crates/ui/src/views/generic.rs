// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::UI_MAX_WIDTH;
use crate::app::{self, AppMsg, ErrorCriticality, FONT_NOTO_EMOJI};
use crate::i18n::fl;
use iced::border::Radius;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::text::Shaping;
use iced::widget::{
    button, center, column, container, mouse_area, opaque, row, rule, scrollable, space, stack,
    text, tooltip, Space, Text, Tooltip,
};
use iced::{Alignment, Color, Element, Length, Shadow, Theme, Vector};
use iced_fonts::bootstrap;

/// "Card" style for a container.
///
/// intended to be used in `container.style` method.
pub(crate) fn card_container_style(theme: &Theme) -> container::Style {
    let mut s = container::rounded_box(theme);
    s.shadow = Shadow {
        color: Color::BLACK,
        offset: Vector::new(1., 2.),
        blur_radius: 3.,
    };
    s
}

/// "Modal" style for a container.
///
/// intended to be used in `container.style` method.
pub(crate) fn modal_container_style(theme: &iced::Theme) -> container::Style {
    let mut s = container::rounded_box(theme).background(theme.palette().background);
    s.border.radius = Radius::new(9.);
    s.shadow = Shadow {
        color: Color::BLACK,
        offset: Vector::new(2., 3.),
        blur_radius: 6.,
    };
    s
}

/// Scrollable scrollbar properties optionally optimized for touch input
///
/// One of `horizontal` or `vertical` arguments must be true.
/// If both are false, it is considered a programmer error and the function will panic.
pub(super) fn optimized_scrollbar_properties(
    horizontal: bool,
    vertical: bool,
    optimize_touch: bool,
) -> Direction {
    let scrollbar = if optimize_touch {
        Scrollbar::default().scroller_width(16).spacing(6.)
    } else {
        Scrollbar::default().spacing(6.)
    };
    match (horizontal, vertical) {
        (false, false) => {
            panic!("At least one of 'horizontal' or 'vertical' needs to be set to true")
        }
        (true, false) => Direction::Horizontal(scrollbar),
        (false, true) => Direction::Vertical(scrollbar),
        (true, true) => Direction::Both {
            horizontal: scrollbar,
            vertical: scrollbar,
        },
    }
}

/// View for a modal supplied by `content`, overlaying base elements supplied by `base`.
/// `on_blur` determines the action when clicking/pressing on the blurred background
pub(crate) fn modal<'a>(
    base: impl Into<Element<'a, AppMsg>>,
    content: impl Into<Element<'a, AppMsg>>,
    on_blur: AppMsg,
) -> Element<'a, AppMsg> {
    stack![
        base.into(),
        mouse_area(center(opaque(content)).style(|_theme| {
            container::Style {
                background: Some(
                    Color {
                        a: 0.9,
                        ..Color::BLACK
                    }
                    .into(),
                ),
                ..container::Style::default()
            }
        }))
        .on_press(on_blur)
    ]
    .into()
}

/// View for nothing at all.
pub(crate) fn view_empty() -> Element<'static, AppMsg> {
    Space::new().into()
}

/// View for an emoji from a character resolved to a emoji glyph by the Noto Emoji font.
#[allow(unused)]
pub(crate) fn view_emoji(emoji: char) -> Text<'static> {
    text(emoji).shaping(Shaping::Advanced).font(FONT_NOTO_EMOJI)
}

/// View for a content separator intended to be used as a dynamic UI element only displayed when the scroll offset
/// is greater then zero (content scrolled down).
#[allow(unused)]
pub(crate) fn view_scrollable_content_sep(scroll_offset: f32) -> Element<'static, AppMsg> {
    if scroll_offset > 0. {
        rule::horizontal(1)
            .style(|theme| {
                let mut style = rule::default(theme);
                let palette = theme.extended_palette();
                style.color = palette.background.weak.color;
                style
            })
            .into()
    } else {
        view_empty()
    }
}

/// View for a text tooltip with text supplied by `tooltip_text` containing any element supplied by `content`.
pub(crate) fn view_text_tooltip<'a>(
    content: impl Into<Element<'a, AppMsg>>,
    tooltip_text: impl text::IntoFragment<'a>,
) -> Tooltip<'a, AppMsg> {
    tooltip(
        content,
        container(text(tooltip_text)).padding(6).style(|theme| {
            let mut s = container::rounded_box(theme).background(Color {
                a: 0.9,
                ..Color::BLACK
            });
            s.shadow = Shadow {
                color: Color::BLACK,
                offset: Vector::new(0., 0.),
                blur_radius: 1.,
            };
            s
        }),
        tooltip::Position::FollowCursor,
    )
}

/// View for a row inside a list
///
/// Intended to be contained in an [iced::widget::Column].
pub(crate) fn view_list_row<'a>(
    left: impl Into<Element<'a, AppMsg>>,
    right: impl Into<Element<'a, AppMsg>>,
) -> Element<'a, AppMsg> {
    row![left.into(), space::horizontal(), right.into()]
        .align_y(Alignment::Center)
        .spacing(6)
        .padding(6)
        .into()
}

/// View for a heading with a certain size
pub(crate) fn view_heading<'a>(heading: impl text::IntoFragment<'a>) -> Text<'a> {
    text(heading).size(24)
}

/// View for a section of UI elements.
///
/// The section has a header where text supplied by `name` is left-aligned
/// and the optional `title-element` is right-aligned.
/// Below both is the `child` element.
pub(crate) fn view_section<'a>(
    name: impl text::IntoFragment<'a>,
    title_element: Option<impl Into<Element<'a, AppMsg>>>,
    child: impl Into<Element<'a, AppMsg>>,
) -> Element<'a, AppMsg> {
    column![
        row![
            view_heading(name),
            space::horizontal(),
            title_element.map(|e| e.into()).unwrap_or(view_empty())
        ]
        .align_y(iced::Alignment::Center),
        child.into()
    ]
    .spacing(12)
    .into()
}

/// View for all supplied `errors`.
///
/// Implemented by visual stack elements
/// indicating how many are stacked.
pub(crate) fn view_errors<'a>(
    errors: impl ExactSizeIterator<Item = &'a app::ErrorReport>,
    optimize_touch: bool,
) -> Element<'a, AppMsg> {
    let n_errors = errors.len();
    const MAX_STACK: usize = 10;

    if n_errors == 0 {
        view_empty()
    } else if n_errors == 1 {
        view_error(errors.last().unwrap(), optimize_touch)
    } else {
        column![
            column((0..n_errors.min(MAX_STACK)).map(|_| {
                rule::horizontal(2)
                    .style(|theme| {
                        let mut s = rule::default(theme);
                        s.color = theme.extended_palette().danger.strong.color;
                        s
                    })
                    .into()
            }))
            .spacing(1),
            view_error(errors.last().unwrap(), optimize_touch)
        ]
        .into()
    }
}

/// View for single error report with visually striking appearance,
/// depending on the error report criticality.
pub(crate) fn view_error(error: &app::ErrorReport, optimize_touch: bool) -> Element<'_, AppMsg> {
    let criticality = match error.criticality {
        ErrorCriticality::NonCritical => fl!("error-noncritical"),
        ErrorCriticality::Critical => fl!("error-critical"),
    };

    container(
        column![
            row![
                text(criticality + " : " + error.short.as_str()),
                space::horizontal(),
                button(bootstrap::x())
                    .style(button::secondary)
                    .on_press(AppMsg::DismissError)
            ]
            .align_y(Alignment::Center)
            .spacing(6),
            scrollable(text(error.detailed.as_str()).size(14))
                .direction(optimized_scrollbar_properties(false, true, optimize_touch))
        ]
        .spacing(6),
    )
    .style(|theme| {
        let mut s = container::bordered_box(theme);
        let extended_palette = theme.extended_palette();
        match error.criticality {
            ErrorCriticality::NonCritical => {
                s.border.color = Color::from_rgb8(209, 160, 0);
                s.background = Some(Color::from_rgb8(156, 144, 103).into());
                s.text_color = Some(extended_palette.danger.base.text);
            }
            ErrorCriticality::Critical => {
                s.border.color = extended_palette.danger.strong.color;
                s.background = Some(extended_palette.danger.weak.color.into());
                s.text_color = Some(extended_palette.danger.base.text);
            }
        }
        s
    })
    .width(Length::Fill)
    .padding(6)
    .into()
}

/// View for a confirmation modal that only sends the suppliced `confirm` message
/// when the user has clicked on the confirm button.
pub(crate) fn view_confirmation_modal<'a>(
    msg: impl text::IntoFragment<'a>,
    confirm: AppMsg,
) -> Element<'a, AppMsg> {
    container(
        column![
            text(msg),
            row![
                button(text(fl!("confirmation-modal-cancel-button")))
                    .on_press(AppMsg::HideModal)
                    .style(button::secondary),
                space::horizontal(),
                button(text(fl!("confirmation-modal-confirm-button")))
                    .on_press(confirm.hide_modal()),
            ]
        ]
        .align_x(Alignment::Center)
        .spacing(6),
    )
    .style(modal_container_style)
    .max_width(UI_MAX_WIDTH - 300.)
    .padding(12)
    .into()
}
