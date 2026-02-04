// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::generic::card_container_style;
use crate::app::{AppMsg, AppNotConnected, Modal, NotConnectedMsg};
use crate::i18n::fl;
use iced::widget::{button, container, row, text, text_input};
use iced::{Alignment, Element, Length};
use iced_fonts::bootstrap;

/// View for the UI when in state [crate::app::AppState::NotConnected]
pub(crate) fn view_app_not_connected(not_connected: &AppNotConnected) -> Element<'_, AppMsg> {
    container(
        row![
            container(
                row![
                    bootstrap::ban(),
                    text_input(
                        fl!("coordinator-address-placeholder").as_str(),
                        not_connected.input_address.as_str()
                    )
                    .on_input(
                        |text| AppMsg::NotConnected(NotConnectedMsg::UpdateInputAddress(text))
                    )
                    .on_submit(AppMsg::NotConnected(NotConnectedMsg::Connect)),
                    button(text(fl!("connect-button")))
                        .on_press(AppMsg::NotConnected(NotConnectedMsg::Connect)),
                ]
                .spacing(6)
                .width(Length::Fill)
                .align_y(Alignment::Center)
            )
            .padding(6)
            .style(card_container_style),
            container(
                button(text(fl!("settings-button")))
                    .on_press(AppMsg::ShowModal(Box::new(Modal::Settings)))
            )
            .padding(6)
        ]
        .spacing(6),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
