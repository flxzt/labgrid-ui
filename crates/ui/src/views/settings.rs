// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::generic::{modal_container_style, view_text_tooltip};
use super::UI_MAX_WIDTH;
use crate::app::{App, AppMsg, ConnectedMsg};
use crate::i18n::{fl, AppLanguage};
use crate::util;
use iced::widget::{button, column, container, pick_list, row, rule, space, text, toggler};
use iced::{padding, Alignment, Element, Length};
use iced_fonts::bootstrap;

/// View for a single settings row.
///
/// intended to be contained in widget [iced::widget::Column]
pub(crate) fn view_settings_row<'a>(
    description: impl text::IntoFragment<'a>,
    action: impl Into<Element<'a, AppMsg>>,
) -> Element<'a, AppMsg> {
    row![text(description), space::horizontal(), action.into()]
        .align_y(Alignment::Center)
        .spacing(6)
        .padding(6)
        .into()
}

/// View for application settings
pub(crate) fn view_settings(app: &App) -> Element<'_, AppMsg> {
    let project_version = util::project_version();

    container(
        column![
            row![
                text(fl!("settings-header")).size(24),
                space::horizontal(),
                button(bootstrap::x()).on_press(AppMsg::HideModal),
            ]
            .spacing(6),
            container(
                column![
                    view_settings_row(
                        fl!("settings-language-pick-label"),
                        pick_list(
                            AppLanguage::LANGS_AVAILABLE,
                            Some(&app.language),
                            AppMsg::ChangeLanguage
                        )
                    ),
                    rule::horizontal(1),
                    view_settings_row(
                        fl!("settings-optimize-touch-label"),
                        toggler(app.optimize_touch).on_toggle(AppMsg::OptimizeTouch)
                    ),
                    rule::horizontal(1),
                    view_settings_row(
                        fl!("settings-venv-dir-label"),
                        row![
                            container(text(app.venv_dir.display().to_string()))
                                .padding(padding::right(5)),
                            view_text_tooltip(
                                button(bootstrap::backspace()).on_press(AppMsg::ChangeVenvDir {
                                    dir: util::default_venv_dir()
                                }),
                                fl!("venv-dir-reset-tooltip")
                            ),
                            view_text_tooltip(
                                button(bootstrap::foldertwo_open()).on_press(AppMsg::Connected(
                                    ConnectedMsg::OpenChangeVenvDirFileDialog {
                                        initial_dir: app.venv_dir.clone()
                                    }
                                )),
                                fl!("settings-venv-dir-pick-tooltip")
                            ),
                        ]
                        .align_y(Alignment::Center)
                        .spacing(1)
                    ),
                    rule::horizontal(1),
                    rule::horizontal(1),
                    view_settings_row(fl!("app-authors-label"), text(util::project_authors())),
                    rule::horizontal(1),
                    view_settings_row(
                        fl!("app-version-label"),
                        row![
                            text(project_version.clone()),
                            view_text_tooltip(
                                button(bootstrap::copy())
                                    .on_press(AppMsg::ClipboardCopy(project_version)),
                                fl!("clipboard-copy-tooltip")
                            )
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center)
                    ),
                    rule::horizontal(1),
                    view_settings_row(
                        "",
                        button(text(fl!("app-quit-label")))
                            .style(button::danger)
                            .on_press(AppMsg::CloseLatestWindow)
                    ),
                ]
                .spacing(6)
                .padding(6)
            )
            .width(Length::Fill)
            .style(container::rounded_box)
        ]
        .spacing(6),
    )
    .style(modal_container_style)
    .max_width(UI_MAX_WIDTH - 200.)
    .padding(12)
    .into()
}
