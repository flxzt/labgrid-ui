// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::app::AppMsg;
use crate::i18n::fl;
use iced::widget::{column, container, text, vertical_space};
use iced::{Alignment, Element, Length};

/// View for the UI when in connecting state
pub(crate) fn view_app_connecting(address: &str) -> Element<'_, AppMsg> {
    container(
        column![
            vertical_space(),
            text(fl!("connecting-msg", address = address)),
            // TODO: spinner
            vertical_space()
        ]
        .width(Length::Fill)
        .align_x(Alignment::Center),
    )
    .into()
}
