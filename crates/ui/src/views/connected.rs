// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::generic::{
    card_container_style, modal_container_style, optimized_scrollbar_properties, view_empty,
    view_heading, view_icon, view_icon_small, view_list_row, view_section, view_text_tooltip,
};
use super::{NONE_ELEMENT, UI_MAX_WIDTH};
use crate::app::{
    AppConnected, AppMsg, ConnectedMsg, Modal, PlaceUi, ResourceUi, TabId, FONT_INCONSOLATA,
};
use crate::connection::ConnectionMsg;
use crate::i18n::fl;
use crate::scripts::{Env, EnvEntry, Script, Scripts};
use crate::{scripts, util};
use iced::border::Radius;
use iced::widget::text::Shaping;
use iced::widget::{
    button, checkbox, column, container, horizontal_rule, horizontal_space, pick_list, row,
    scrollable, text, text_input, Space,
};
use iced::{padding, Alignment, Color, Element, Length};
use iced_aw::{TabBarPosition, TabLabel, Tabs};
use iced_fonts::Bootstrap;
use labgrid_ui_core::types::{Place, Reservation, Resource, ResourceMatch};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// View for a card element that contains general info and basic control for the supplied place
pub(crate) fn view_place_general_info<'a>(
    place: &'a Place,
    ui: &'a PlaceUi,
) -> Element<'a, AppMsg> {
    let acquired_by_row: Element<'_, AppMsg> = if let Some(acquired) = &place.acquired {
        view_list_row(
            text(fl!("labgrid-place-acquired-by-label") + " : "),
            text(acquired),
        )
    } else {
        view_list_row(view_empty(), text(fl!("labgrid-place-not-acquired-label")))
    };
    let tags_row: Element<'a, AppMsg> = if let Some(tag) = &ui.add_tag_text {
        row![
            row![
                text_input(&fl!("labgrid-place-add-tag-placeholder"), &tag.0)
                    .on_input(
                        |text| AppMsg::Connected(ConnectedMsg::UpdateAddPlaceTagText {
                            place_name: place.name.clone(),
                            text
                        })
                    )
                    .width(Length::FillPortion(1)),
                text(" = "),
                text_input(&fl!("labgrid-place-add-tag-value-placeholder"), &tag.1)
                    .on_input(
                        |text| AppMsg::Connected(ConnectedMsg::UpdateAddPlaceTagValueText {
                            place_name: place.name.clone(),
                            text,
                        })
                    )
                    .width(Length::FillPortion(1)),
            ]
            .spacing(1)
            .width(Length::Fill)
            .align_y(Alignment::Center),
            row![
                view_text_tooltip(
                    button(view_icon(Bootstrap::Backspace)).on_press(AppMsg::Connected(
                        ConnectedMsg::ClearAddPlaceTagText {
                            place_name: place.name.clone()
                        }
                    )),
                    fl!("text-input-clear-tooltip")
                ),
                view_text_tooltip(
                    button(view_icon(Bootstrap::Plus)).on_press(AppMsg::ConnectionMsg(
                        ConnectionMsg::AddPlaceTag {
                            place_name: place.name.clone(),
                            tag: tag.to_owned()
                        }
                    )),
                    fl!("labgrid-place-add-tag-tooltip")
                ),
                view_text_tooltip(
                    button(view_icon(Bootstrap::X)).on_press(AppMsg::Connected(
                        ConnectedMsg::CloseAddPlaceTag {
                            place_name: place.name.clone()
                        }
                    )),
                    fl!("labgrid-place-close-add-tag-tooltip")
                )
            ]
            .spacing(1)
            .align_y(Alignment::Center),
        ]
        .align_y(Alignment::Center)
        .spacing(6)
        .padding(6)
        .into()
    } else {
        view_list_row(
            text(fl!("labgrid-place-tags-label") + " : "),
            row![
                row(place.tags.iter().map(|t| view_tag(&place.name, (t.0, t.1))))
                    .spacing(3)
                    .wrap(),
                view_text_tooltip(
                    button(view_icon(Bootstrap::Plus)).on_press(AppMsg::Connected(
                        ConnectedMsg::ShowAddPlaceTag {
                            place_name: place.name.clone()
                        }
                    )),
                    fl!("labgrid-place-add-tag-tooltip")
                )
            ]
            .spacing(3)
            .align_y(Alignment::Center),
        )
    };
    column![
        view_list_row(
            text(fl!("labgrid-place-name-label") + " : "),
            text(&place.name)
        ),
        horizontal_rule(1),
        view_list_row(
            text(fl!("labgrid-place-comment-label") + " : "),
            text(&place.comment)
        ),
        horizontal_rule(1),
        acquired_by_row,
        horizontal_rule(1),
        tags_row,
    ]
    .into()
}

/// View for the tab that views the supplied places
pub(crate) fn view_places_tab<'a>(
    places: &'a [(Place, PlaceUi)],
    add_place_text: &'a str,
    optimize_touch: bool,
) -> Element<'a, AppMsg> {
    let places_list = row(places.iter().map(|(p, ui)| view_place(p, ui)))
        .spacing(12.)
        .padding(padding::bottom(12))
        .wrap();
    container(view_section(
        fl!("labgrid-places-label"),
        Some(
            row![
                view_text_tooltip(
                    button(view_icon(Bootstrap::Clipboard))
                        .on_press(AppMsg::Connected(ConnectedMsg::ClipboardPasteAddPlaceName)),
                    fl!("clipboard-paste-tooltip")
                ),
                text_input(
                    fl!("labgrid-place-add-placeholder").as_str(),
                    add_place_text
                )
                .on_input(|text| AppMsg::Connected(ConnectedMsg::UpdateAddPlaceName(text))),
                view_text_tooltip(
                    button(view_icon(Bootstrap::Backspace)).on_press(AppMsg::Connected(
                        ConnectedMsg::UpdateAddPlaceName(String::new())
                    )),
                    fl!("text-input-clear-tooltip")
                ),
                Space::new(6, 0),
                button(text(fl!("labgrid-place-add-button"))).on_press(AppMsg::ConnectionMsg(
                    ConnectionMsg::AddPlace {
                        name: add_place_text.to_string()
                    }
                ))
            ]
            .spacing(1),
        ),
        scrollable(places_list)
            .direction(optimized_scrollbar_properties(false, true, optimize_touch))
            .width(Length::Fill),
    ))
    .padding(6)
    .into()
}

/// View for the tab viewing all supplied reservations
pub(crate) fn view_reservations_tab<'a>(
    reservations: impl IntoIterator<Item = &'a Reservation>,
    optimize_touch: bool,
) -> Element<'a, AppMsg> {
    let reservations_list = row(reservations.into_iter().map(view_reservation))
        .spacing(12.)
        .padding(padding::bottom(12))
        .wrap();

    container(view_section(
        fl!("labgrid-reservations-label"),
        NONE_ELEMENT,
        scrollable(reservations_list)
            .direction(optimized_scrollbar_properties(false, true, optimize_touch))
            .width(Length::Fill),
    ))
    .padding(6)
    .into()
}

/// View for the tab viewing all supplied resources
pub(crate) fn view_resources_tab<'a>(
    resources: impl IntoIterator<Item = &'a (Resource, ResourceUi)>,
    only_show_available: bool,
    optimize_touch: bool,
) -> Element<'a, AppMsg> {
    let unnamed_group: String = fl!("labgrid-resources-no-exporter-name");
    // BTreeMap is automatically sorted by keys
    let mut grouped_resources: BTreeMap<String, Vec<&(Resource, ResourceUi)>> = BTreeMap::new();

    for r in resources.into_iter() {
        let exporter_name =
            r.0.path
                .exporter_name
                .clone()
                .unwrap_or(unnamed_group.clone());
        if let Some(v) = grouped_resources.get_mut(&exporter_name) {
            v.push(r);
        } else {
            grouped_resources.insert(exporter_name, vec![r]);
        }
    }

    let resources_list = column(grouped_resources.into_iter().map(|(n, mut resources)| {
        resources.sort_by(|first, second| first.0.path.numeric_cmp(&second.0.path));

        view_section(
            n,
            NONE_ELEMENT,
            column(resources.into_iter().filter_map(|(resource, ui)| {
                if only_show_available {
                    if resource.available {
                        Some(view_resource(resource, ui))
                    } else {
                        None
                    }
                } else {
                    Some(view_resource(resource, ui))
                }
            }))
            .spacing(6),
        )
    }))
    .width(Length::Fill)
    .spacing(12);

    container(view_section(
        fl!("labgrid-resources-label"),
        Some(
            checkbox(
                fl!("labgrid-resources-only-show-available-checkbox"),
                only_show_available,
            )
            .on_toggle(|show| AppMsg::Connected(ConnectedMsg::ResourcesOnlyShowAvailable(show))),
        ),
        scrollable(resources_list)
            .direction(optimized_scrollbar_properties(false, true, optimize_touch))
            .width(Length::Fill),
    ))
    .padding(6)
    .into()
}

/// View for the tab viewing all scripts contained in the supplied `connected` app state
pub(crate) fn view_scripts_tab(
    connected: &AppConnected,
    optimize_touch: bool,
) -> Element<'_, AppMsg> {
    column![
        row![
            column![
                view_heading(fl!("scripts-env-label")),
                view_env(&connected.scripts.env, &connected.places)
            ]
            .spacing(12)
            .padding(6),
            view_scripts(&connected.scripts, &connected.script_status, optimize_touch)
        ]
        .height(Length::FillPortion(1)),
        view_section(
            fl!("script-output-label"),
            Some(
                row![
                    view_text_tooltip(
                        button(view_icon(Bootstrap::Copy))
                            .on_press(AppMsg::ClipboardCopy(connected.script_out.clone())),
                        fl!("clipboard-copy-tooltip")
                    ),
                    view_text_tooltip(
                        button(view_icon(Bootstrap::Backspace))
                            .on_press(AppMsg::Connected(ConnectedMsg::ScriptOutClear)),
                        fl!("script-output-clear-tooltip")
                    ),
                    if connected.script_show_output {
                        // TODO: How to use icons here without static lifetime issue?
                        button(text(fl!("script-output-hide-label")))
                            .on_press(AppMsg::Connected(ConnectedMsg::ScriptOutHide))
                    } else {
                        button(text(fl!("script-output-show-label")))
                            .on_press(AppMsg::Connected(ConnectedMsg::ScriptOutShow))
                    }
                ]
                .spacing(1)
            ),
            if connected.script_show_output {
                view_process_output(
                    &connected.script_out,
                    Length::FillPortion(1),
                    optimize_touch,
                )
            } else {
                view_empty()
            }
        )
    ]
    .spacing(12)
    .into()
}

/// View for the supplied environment with controls
/// that can modify specific [EnvEntry]'s through custom widgets.
///
/// e.g. [EnvEntry::LgPlace] can be modified by picking a directory,
/// [EnvEntry::LgPlace] can be modified through a pick list that lists available places.
pub(crate) fn view_env<'a>(env: &'a Env, places: &'a [(Place, PlaceUi)]) -> Element<'a, AppMsg> {
    const ENTRY_WIDTH: f32 = 350.;
    let places_names: Vec<&'a String> = places.iter().map(|(p, _)| &p.name).collect();
    let selected_place = env.get(&EnvEntry::LgPlace);
    let lg_env_val = env
        .get(&EnvEntry::LgEnv)
        .map(|s| s.to_string())
        .unwrap_or_default();

    column![
        container(
            row![
                text(EnvEntry::LgPlace.as_env_var() + " = "),
                horizontal_space(),
                pick_list(places_names, selected_place, |p| {
                    AppMsg::Connected(ConnectedMsg::ScriptsEnvUpdate {
                        entry: EnvEntry::LgPlace,
                        value: p.to_string(),
                    })
                }),
                button(view_icon(Bootstrap::Backspace)).on_press(AppMsg::Connected(
                    ConnectedMsg::ScriptsEnvClear {
                        entry: EnvEntry::LgPlace
                    }
                ))
            ]
            .spacing(6)
            .padding(3)
            .width(ENTRY_WIDTH)
            .align_y(Alignment::Center)
        )
        .style(container::rounded_box),
        container(
            row![
                text(EnvEntry::LgEnv.as_env_var() + " = "),
                horizontal_space(),
                text(lg_env_val.clone()),
                button(view_icon(Bootstrap::FoldertwoOpen)).on_press(AppMsg::Connected(
                    ConnectedMsg::ScriptsEnvOpenLgEnvFileDialog {
                        initial_file: PathBuf::from(lg_env_val)
                    }
                )),
                button(view_icon(Bootstrap::Backspace)).on_press(AppMsg::Connected(
                    ConnectedMsg::ScriptsEnvClear {
                        entry: EnvEntry::LgEnv
                    }
                ))
            ]
            .spacing(6)
            .padding(3)
            .width(ENTRY_WIDTH)
            .align_y(Alignment::Center)
        )
        .style(container::rounded_box)
    ]
    .spacing(6)
    .into()
}

/// View for the supplied scripts.
///
/// `script_status` is the state for the single current script.
/// E.g. if it's path matches with one of the scripts, the script element will display running, finished
/// with the exit-code, .. depending on the status
pub(crate) fn view_scripts<'a>(
    scripts: &'a Scripts,
    script_status: &'a scripts::ScriptStatus,
    optimize_touch: bool,
) -> Element<'a, AppMsg> {
    let scripts_dir = scripts.dir();
    let scripts_iter = scripts.iter();
    let scripts_dir_str = scripts_dir.display().to_string();
    let scripts_list: Element<'a, AppMsg> = if scripts_iter.len() == 0 {
        container(text(fl!("scripts-none-found-msg")))
            .padding(12)
            .into()
    } else {
        row(scripts_iter.map(|s| view_script(s, script_status)))
            .spacing(12.)
            .padding(padding::bottom(12))
            .wrap()
            .into()
    };

    container(column![view_section(
        fl!("scripts-label"),
        Some(
            row![
                container(text(scripts_dir_str)).padding(padding::right(5)),
                view_text_tooltip(
                    button(view_icon(Bootstrap::FoldertwoOpen)).on_press(AppMsg::Connected(
                        ConnectedMsg::OpenChangeScriptsDirDialog {
                            initial_dir: scripts_dir.to_owned()
                        }
                    )),
                    fl!("scripts-dir-pick-tooltip")
                ),
                view_text_tooltip(
                    button(view_icon(Bootstrap::Backspace)).on_press(AppMsg::ChangeScriptsDir {
                        dir: util::default_scripts_dir()
                    }),
                    fl!("scripts-dir-reset-tooltip")
                ),
                view_text_tooltip(
                    button(view_icon(Bootstrap::ArrowClockwise))
                        .on_press(AppMsg::Connected(ConnectedMsg::RescanScriptsDir)),
                    fl!("scripts-dir-rescan-tooltip")
                ),
            ]
            .align_y(Alignment::Center)
            .spacing(1)
        ),
        scrollable(scripts_list)
            .direction(optimized_scrollbar_properties(false, true, optimize_touch))
            .width(Length::Fill),
    )])
    .padding(6)
    .into()
}

/// Creates a view for a script.
///
/// The path must point to a existing python script,
/// it is a programmer error if it is not checked,
/// and the function might panic.
pub(crate) fn view_script<'a>(
    script: &'a Script,
    script_status: &'a scripts::ScriptStatus,
) -> Element<'a, AppMsg> {
    let filename = script
        .path()
        .file_name()
        .expect("Path to script without name")
        .to_string_lossy()
        .to_string();
    let script_execute_abort_button = match script_status {
        scripts::ScriptStatus::Running {
            script: running, ..
        } if script == running => button(text(fl!("script-abort-button")))
            .style(button::danger)
            .on_press(AppMsg::Connected(ConnectedMsg::AbortScript)),

        _ => button(text(fl!("script-execute-button"))).on_press(AppMsg::Connected(
            ConnectedMsg::ExecuteScript {
                script: script.clone(),
            },
        )),
    };
    let status_element: Element<'a, AppMsg> = match script_status {
        scripts::ScriptStatus::Running {
            script: running, ..
        } if script == running => text(fl!("script-status-running")).into(),
        scripts::ScriptStatus::Finished {
            script: finished,
            exit_code,
        } if script == finished => container(text(fl!(
            "script-status-finished",
            code = exit_code.to_string()
        )))
        .style(|theme: &iced::Theme| {
            let mut s = container::rounded_box(theme);
            if *exit_code == 0 {
                s = s.background(Color::from_rgb8(134, 186, 104));
            } else {
                s = s.background(theme.extended_palette().danger.weak.color);
            }
            s
        })
        .padding(6)
        .into(),
        _ => text(fl!("script-status-none")).into(),
    };

    container(column![
        view_list_row(text(fl!("script-label") + " : "), text(filename)),
        horizontal_rule(1),
        view_list_row(text(fl!("script-status-label")), status_element),
        horizontal_rule(1),
        view_list_row(view_empty(), script_execute_abort_button)
    ])
    .style(card_container_style)
    // Must be a fixed width for predictable layout and to avoid panic when using horizontal_space
    .width(340)
    .padding(6)
    .into()
}

/// View for a process output that displays the content of `out`
/// in a monospace font and in a look that emulates a terminal.
pub(crate) fn view_process_output<'a>(
    out: &'a str,
    height: impl Into<Length>,
    optimize_touch: bool,
) -> Element<'a, AppMsg> {
    container(Element::<'a, AppMsg>::from(
        scrollable(
            text(out)
                .shaping(Shaping::Advanced)
                .font(FONT_INCONSOLATA)
                .style(|_| text::Style {
                    color: Some(Color::WHITE),
                }),
        )
        .direction(optimized_scrollbar_properties(false, true, optimize_touch))
        .width(Length::Fill)
        .height(Length::Fill),
    ))
    .style(|theme| {
        let mut s = card_container_style(theme);
        s.background = Some(Color::BLACK.into());
        s
    })
    .padding(12)
    .width(Length::Fill)
    .height(height)
    .max_height(600)
    .into()
}

/// View a single supplied place.
/// `ui` holds state about the place ui, e.g. whether the place details should be shown or not.
pub(crate) fn view_place<'a>(place: &'a Place, ui: &'a PlaceUi) -> Element<'a, AppMsg> {
    let delete_button: Element<'_, AppMsg> = button(text(fl!("labgrid-place-delete-button")))
        .on_press(AppMsg::ShowModal(Box::new(Modal::Confirmation {
            msg: fl!(
                "labgrid-place-delete-confirmation-msg",
                place = place.name.clone()
            ),
            confirm: AppMsg::ConnectionMsg(ConnectionMsg::DeletePlace {
                name: place.name.clone(),
            }),
        })))
        .style(button::danger)
        .into();
    let acquired_release_button: Element<'_, AppMsg> = if place.acquired.is_some() {
        button(text(fl!("labgrid-place-release-label")))
            .on_press(AppMsg::ConnectionMsg(ConnectionMsg::ReleasePlace {
                name: place.name.clone(),
            }))
            .style(button::danger)
            .into()
    } else {
        button(text(fl!("labgrid-place-acquire-button")))
            .on_press(AppMsg::ConnectionMsg(ConnectionMsg::AcquirePlace {
                name: place.name.clone(),
            }))
            .into()
    };

    container(column![
        view_place_general_info(place, ui),
        horizontal_rule(1),
        view_list_row(
            button(text(fl!("show-details-button")))
                .style(button::secondary)
                .on_press(AppMsg::ShowModal(Box::new(Modal::PlaceDetails {
                    place_name: place.name.clone()
                }))),
            row![delete_button, acquired_release_button]
                .align_y(Alignment::Center)
                .spacing(6)
        )
    ])
    .style(card_container_style)
    // Must be a fixed width for predictable layout and to avoid panic when using horizontal_space
    .width(340)
    .padding(6)
    .into()
}

/// View for a single reservation
pub(crate) fn view_reservation(reservation: &Reservation) -> Element<'_, AppMsg> {
    container(column![
        view_list_row(
            text(fl!("labgrid-reservation-owner-label") + " : "),
            text(&reservation.owner)
        ),
        horizontal_rule(1),
        view_list_row(
            text(fl!("labgrid-reservation-token-label") + " : "),
            row![
                text(&reservation.token),
                view_text_tooltip(
                    button(view_icon(Bootstrap::Copy))
                        .style(button::secondary)
                        .on_press(AppMsg::ClipboardCopy(reservation.token.clone())),
                    fl!("clipboard-copy-tooltip")
                )
            ]
            .align_y(Alignment::Center)
            .spacing(6)
        ),
        horizontal_rule(1),
        view_list_row(
            text(fl!("labgrid-reservation-prio-label") + " : "),
            text(reservation.prio.to_string())
        ),
        horizontal_rule(1),
        view_list_row(
            text(fl!("labgrid-reservation-filters-label") + " : "),
            text(format!("{:?}", reservation.filters))
        ),
        view_list_row(
            view_empty(),
            button(text(fl!("labgrid-reservation-cancel-label")))
                .style(button::danger)
                .on_press(AppMsg::ConnectionMsg(ConnectionMsg::CancelReservation {
                    token: reservation.token.clone()
                }))
        ),
    ])
    .style(card_container_style)
    // Must be a fixed width for predictable layout and to avoid panic when using horizontal_space
    .width(340)
    .padding(6)
    .into()
}

/// View for a single resource.
///
/// `ui` holds state about the resource UI, e.g. whether details about the resource should be shown
pub(crate) fn view_resource<'a>(resource: &'a Resource, ui: &'a ResourceUi) -> Element<'a, AppMsg> {
    let resource_path_str = format!(
        "{}/{}/{}[/{}]",
        resource.path.exporter_name.clone().unwrap_or_default(),
        resource.path.group_name,
        resource.cls,
        resource.path.resource_name
    );
    let copy_clipboard_msg = format!(
        "{}/{}/{}/{}",
        resource.path.exporter_name.clone().unwrap_or_default(),
        resource.path.group_name,
        resource.cls,
        resource.path.resource_name
    );
    let copy_name_to_clipboard_button = view_text_tooltip(
        button(view_icon(Bootstrap::Copy))
            .style(button::secondary)
            .on_press(AppMsg::ClipboardCopy(copy_clipboard_msg)),
        fl!("clipboard-copy-tooltip"),
    );
    let availability_widget = view_text_tooltip(
        checkbox("", resource.available),
        fl!("labgrid-resource-availability-tooltip"),
    );

    if ui.show_details {
        container(column![
            view_list_row(
                text(resource_path_str),
                row![
                    copy_name_to_clipboard_button,
                    availability_widget,
                    button(text(fl!("hide-details-button"))).on_press(AppMsg::Connected(
                        ConnectedMsg::HideResourceDetails(resource.path.clone())
                    ))
                ]
                .align_y(Alignment::Center)
                .spacing(6)
            ),
            horizontal_rule(1),
            view_list_row(
                text(fl!("labgrid-resource-acquired-label") + " : "),
                text(&resource.acquired)
            ),
            horizontal_rule(1),
            // TODO: improve params view
            view_list_row(
                text(fl!("labgrid-resource-params-label") + " : "),
                text(format!("{:?}", resource.params))
            ),
            horizontal_rule(1),
            // TODO: improve extra view
            view_list_row(
                text(fl!("labgrid-resource-extra-label") + " : "),
                text(format!("{:?}", resource.extra))
            ),
        ])
        .style(card_container_style)
        .into()
    } else {
        container(view_list_row(
            text(resource_path_str),
            row![
                copy_name_to_clipboard_button,
                availability_widget,
                button(text(fl!("show-details-button")))
                    .style(button::secondary)
                    .on_press(AppMsg::Connected(ConnectedMsg::ShowResourceDetails(
                        resource.path.clone()
                    )))
            ]
            .align_y(Alignment::Center)
            .spacing(6),
        ))
        .style(card_container_style)
        .into()
    }
}

/// View for a single place tag.
pub(crate) fn view_tag<'a>(place_name: &'a str, tag: (&'a str, &'a str)) -> Element<'a, AppMsg> {
    container(
        row![
            text(tag.0).size(12),
            text("=").size(12),
            text(tag.1).size(12),
            button(view_icon_small(Bootstrap::X))
                .padding(2)
                .style(button::secondary)
                .on_press(AppMsg::ShowModal(Box::new(Modal::Confirmation {
                    msg: fl!("labgrid-place-delete-tag-confirmation-msg", tag = tag.0),
                    confirm: AppMsg::ConnectionMsg(ConnectionMsg::DeletePlaceTag {
                        place_name: place_name.to_string(),
                        tag: tag.0.to_string()
                    })
                })))
        ]
        .align_y(Alignment::Center)
        .spacing(2),
    )
    .style(|theme| {
        let mut s = container::bordered_box(theme);
        s.border.radius = Radius::new(2);
        s
    })
    .padding(3)
    .into()
}

/// View for a resource match for a place as reported by labgrid's client out stream
pub(crate) fn view_resource_match<'a>(
    place: &'a Place,
    resource_match: &'a ResourceMatch,
) -> Element<'a, AppMsg> {
    let (match_pattern, match_display) = if let Some(name) = &resource_match.name {
        (
            format!(
                "{}/{}/{}/{}",
                resource_match.exporter, resource_match.group, resource_match.cls, name
            ),
            format!(
                "{}/{}/{}/[{}]",
                resource_match.exporter, resource_match.group, resource_match.cls, name
            ),
        )
    } else {
        (
            format!(
                "{}/{}/{}",
                resource_match.exporter, resource_match.group, resource_match.cls
            ),
            format!(
                "{}/{}/{}",
                resource_match.exporter, resource_match.group, resource_match.cls
            ),
        )
    };
    container(view_list_row(
        text(match_display),
        row![
            view_text_tooltip(
                button(view_icon(Bootstrap::Copy))
                    .style(button::secondary)
                    .on_press(AppMsg::ClipboardCopy(match_pattern.clone())),
                fl!("clipboard-copy-tooltip")
            ),
            button(text(fl!("labgrid-place-resource-match-delete-button")))
                .style(button::danger)
                .on_press(AppMsg::ConnectionMsg(ConnectionMsg::DeletePlaceMatch {
                    place_name: place.name.clone(),
                    pattern: match_pattern,
                },))
        ]
        .spacing(6),
    ))
    .style(card_container_style)
    .into()
}

/// View for a acquired resource in a place as reported by labgrid's client out stream
pub(crate) fn view_acquired_resource(acquired_resource: String) -> Element<'static, AppMsg> {
    container(view_list_row(
        text(acquired_resource.clone()),
        view_text_tooltip(
            button(view_icon(Bootstrap::Copy))
                .style(button::secondary)
                .on_press(AppMsg::ClipboardCopy(acquired_resource)),
            fl!("clipboard-copy-tooltip"),
        ),
    ))
    .style(card_container_style)
    .into()
}

/// View for the place details modal that gets displayed when the place UI state `show_details` is set.
pub(crate) fn view_place_details<'a>(
    place: &'a Place,
    ui: &'a PlaceUi,
    optimize_touch: bool,
    add_place_match_text: &'a str,
) -> Element<'a, AppMsg> {
    let place_name = &place.name;
    let resource_matches_list = column(place.matches.iter().map(|m| view_resource_match(place, m)))
        .spacing(6)
        .padding(6);
    let resources_acquired_list = column(
        place
            .acquired_resources
            .iter()
            .map(|n| view_acquired_resource(n.to_owned())),
    )
    .spacing(6)
    .padding(6);

    container(
        column![
            row![
                text(fl!("labgrid-place-details-header", place = place_name)).size(24),
                horizontal_space(),
                button(view_icon(Bootstrap::X)).on_press(AppMsg::HideModal)
            ],
            scrollable(
                column![
                    container(view_place_general_info(place, ui))
                        .style(card_container_style)
                        .padding(6),
                    view_section(
                        fl!("labgrid-place-resource-matches-header"),
                        Some(
                            row![
                                view_text_tooltip(
                                    button(view_icon(Bootstrap::Clipboard)).on_press(
                                        AppMsg::Connected(
                                            ConnectedMsg::ClipboardPasteAddPlaceMatchPattern
                                        )
                                    ),
                                    fl!("clipboard-paste-tooltip")
                                ),
                                text_input(
                                    fl!("labgrid-place-resource-match-add-placeholder-text")
                                        .as_str(),
                                    add_place_match_text
                                )
                                .on_input(
                                    |text| AppMsg::Connected(
                                        ConnectedMsg::UpdateAddPlaceMatchPattern(text)
                                    )
                                ),
                                view_text_tooltip(
                                    button(view_icon(Bootstrap::Backspace)).on_press(
                                        AppMsg::Connected(
                                            ConnectedMsg::UpdateAddPlaceMatchPattern(String::new())
                                        )
                                    ),
                                    fl!("text-input-clear-tooltip")
                                ),
                                Space::new(6, 0),
                                button(text(fl!("labgrid-place-resource-match-add-button")))
                                    .on_press(AppMsg::ConnectionMsg(
                                        ConnectionMsg::AddPlaceMatch {
                                            place_name: place.name.clone(),
                                            pattern: add_place_match_text.to_string()
                                        }
                                    ))
                            ]
                            .spacing(1)
                        ),
                        resource_matches_list,
                    ),
                    view_section(
                        fl!("labgrid-place-resource-acquired-header"),
                        NONE_ELEMENT,
                        resources_acquired_list,
                    )
                ]
                .spacing(12)
            )
            .direction(optimized_scrollbar_properties(false, true, optimize_touch))
        ]
        .spacing(12),
    )
    .style(modal_container_style)
    .max_width(UI_MAX_WIDTH)
    .padding(12)
    .into()
}

/// View for the "connected" app state
pub(crate) fn view_app_connected(
    connected: &AppConnected,
    optimize_touch: bool,
) -> Element<'_, AppMsg> {
    column![
        row![
            container(
                row![
                    view_icon(Bootstrap::Link),
                    text(fl!(
                        "connected-to-coordinator-label",
                        address = connected.address.as_str()
                    )),
                    horizontal_space(),
                    view_text_tooltip(
                        button(view_icon(Bootstrap::ArrowClockwise))
                            .on_press(AppMsg::Connected(ConnectedMsg::Refresh)),
                        fl!("refresh-ui-tooltip")
                    ),
                    button(text(fl!("disconnect-button")))
                        .on_press(AppMsg::Connected(ConnectedMsg::Disconnect)),
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
        Tabs::new(|id| AppMsg::Connected(ConnectedMsg::TabSelected(id)))
            .push(
                TabId::Places,
                TabLabel::Text(fl!("labgrid-places-label")),
                container(view_places_tab(
                    &connected.places,
                    &connected.add_place_text,
                    optimize_touch
                ))
                .padding(padding::top(6))
            )
            .push(
                TabId::Reservations,
                TabLabel::Text(fl!("labgrid-reservations-label")),
                container(view_reservations_tab(
                    &connected.reservations,
                    optimize_touch
                ))
                .padding(padding::top(6))
            )
            .push(
                TabId::Resources,
                TabLabel::Text(fl!("labgrid-resources-label")),
                container(view_resources_tab(
                    &connected.resources,
                    connected.resources_only_show_available,
                    optimize_touch
                ))
                .padding(padding::top(6))
            )
            .push(
                TabId::Scripts,
                TabLabel::Text(fl!("scripts-label")),
                container(view_scripts_tab(connected, optimize_touch)).padding(padding::top(6))
            )
            .set_active_tab(&connected.active_tab)
            .tab_bar_position(TabBarPosition::Top)
            .tab_label_spacing(6.)
            .tab_label_padding(6.)
    ]
    .spacing(6)
    .into()
}
