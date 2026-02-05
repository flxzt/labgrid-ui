// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

// Modules
pub(crate) mod connected;
pub(crate) mod connecting;
pub(crate) mod generic;
pub(crate) mod notconnected;
pub(crate) mod settings;

// Re-Exports
//pub(crate) use connected::*;
//pub(crate) use connecting::*;
//pub(crate) use generic::*;
//pub(crate) use notconnected::*;
//pub(crate) use settings::*;

// Imports
use crate::app::{App, AppMsg, AppState, Modal};
use connected::{view_app_connected, view_place_details};
use connecting::view_app_connecting;
use generic::{modal, view_confirmation_modal, view_errors};
use iced::widget::{column, container};
use iced::{Element, Length};
use notconnected::view_app_not_connected;
use settings::view_settings;
use tracing::error;

/// The maximum width for the all base UI element and all modals
pub(crate) const UI_MAX_WIDTH: f32 = 1000.;
/// Shortcut for [Option::None] typed as `Element<AppMsg>`
pub(crate) const NONE_ELEMENT: Option<Element<AppMsg>> = None::<Element<AppMsg>>;
/// Shortcut for [Option::None] typed as `&str`
#[allow(unused)]
pub(crate) const NONE_STR: Option<&'static str> = None::<&'static str>;

/// View for the entire application
pub(crate) fn view_app(app: &App) -> Element<'_, AppMsg> {
    let state_content = match &app.state {
        AppState::NotConnected(not_connected) => view_app_not_connected(not_connected),
        AppState::Connecting { address } => view_app_connecting(address),
        AppState::Connected(connected) => view_app_connected(connected, app.optimize_touch),
    };
    let content = container(column![
        state_content,
        view_errors(app.errors.iter(), app.optimize_touch)
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(6);

    match &app.modal {
        Modal::None => content.into(),
        Modal::Settings => modal(content, view_settings(app), AppMsg::HideModal),
        Modal::PlaceDetails { place_name } => {
            if let AppState::Connected(connected) = &app.state {
                if let Some((place, ui)) = connected.place_by_name(place_name) {
                    modal(
                        content,
                        view_place_details(
                            place,
                            ui,
                            app.optimize_touch,
                            &connected.add_place_match_text,
                        ),
                        AppMsg::HideModal,
                    )
                } else {
                    error!(
                        "Can't show place details modal, place with name '{place_name}' not found"
                    );
                    content.into()
                }
            } else {
                error!("Can't show place details modal, not connected");
                content.into()
            }
        }
        Modal::Confirmation { msg, confirm } => modal(
            content,
            view_confirmation_modal(msg, confirm.clone()),
            AppMsg::HideModal,
        ),
    }
}
