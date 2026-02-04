// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::config::{self, Config};
use crate::connection::{self, ConnectionEvent, ConnectionMsg, ConnectionSender};
use crate::i18n::{self, fl, AppLanguage};
use crate::scripts::{EnvEntry, Script, ScriptStatus, Scripts};
use crate::views::{self};
use crate::{scripts, util, Args};
use anyhow::Context;
use arboard::Clipboard;
use iced::{window, Font, Size, Subscription, Task};
use iced_fonts::BOOTSTRAP_FONT_BYTES;
use labgrid_ui_core::types::{self, Place, Reservation, Resource};
use std::path::{Path, PathBuf};
use tracing::{debug, error, warn};

#[allow(unused)]
pub(crate) const FONT_CANTARELL: Font = Font::with_name("Cantarell");
#[allow(unused)]
pub(crate) const FONT_NOTO_EMOJI: Font = Font::with_name("Noto Emoji");
#[allow(unused)]
pub(crate) const FONT_INCONSOLATA: Font = Font::with_name("Inconsolata");

/// Identifier for the current selected tab page.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(crate) enum TabId {
    #[default]
    Places,
    Reservations,
    Resources,
    Scripts,
}

/// Top-level app messages.
///
/// Emitted by the UI elements, handled by the app update routines.
#[derive(Debug, Clone)]
pub(crate) enum AppMsg {
    None,
    ChangeLanguage(AppLanguage),
    OptimizeTouch(bool),
    ClipboardCopy(String),
    SaveConfig,
    CloseLatestWindow,
    CloseWindow(window::Id),
    ShowModal(Box<Modal>),
    HideModal,
    WithHideModal(Box<Self>),
    DismissError,
    ChangeVenvDir { dir: PathBuf },
    ChangeScriptsDir { dir: PathBuf },
    ConnectionMsg(ConnectionMsg),
    ConnectionEvent(ConnectionEvent),
    NotConnected(NotConnectedMsg),
    Connected(ConnectedMsg),
}

impl AppMsg {
    /// Wrap the app message with a hide modal message.
    ///
    /// Useful when an action should be executed in addition to closing the current active modal.
    pub(crate) fn hide_modal(self) -> Self {
        Self::WithHideModal(Box::new(self))
    }
}

/// Message when the app is in "not connected" state.
#[derive(Debug, Clone)]
pub(crate) enum NotConnectedMsg {
    Connect,
    UpdateInputAddress(String),
}

/// Message when the app is in "connected" state.
#[derive(Debug, Clone)]
pub(crate) enum ConnectedMsg {
    Disconnect,
    Refresh,
    TabSelected(TabId),
    UpdateAddPlaceName(String),
    ClipboardPasteAddPlaceName,
    ShowResourceDetails(types::Path),
    ResourcesOnlyShowAvailable(bool),
    HideResourceDetails(types::Path),
    UpdateAddPlaceMatchPattern(String),
    ClipboardPasteAddPlaceMatchPattern,
    ShowAddPlaceTag {
        place_name: String,
    },
    CloseAddPlaceTag {
        place_name: String,
    },
    UpdateAddPlaceTagText {
        place_name: String,
        text: String,
    },
    UpdateAddPlaceTagValueText {
        place_name: String,
        text: String,
    },
    ClearAddPlaceTagText {
        place_name: String,
    },
    OpenChangeScriptsDirDialog {
        initial_dir: PathBuf,
    },
    OpenChangeVenvDirFileDialog {
        initial_dir: PathBuf,
    },
    RescanScriptsDir,
    ExecuteScript {
        script: Script,
    },
    AbortScript,
    ScriptFinished {
        script: Script,
        exit_code: i32,
        stdout: String,
        stderr: String,
    },
    ScriptExecutionFailed {
        script: Script,
        err: String,
    },
    ScriptsEnvUpdate {
        entry: EnvEntry,
        value: String,
    },
    ScriptsEnvClear {
        entry: EnvEntry,
    },
    ScriptsEnvOpenLgEnvFileDialog {
        initial_file: PathBuf,
    },
    ScriptOutShow,
    ScriptOutHide,
    ScriptOutClear,
}

/// Starts the entire application.
///
/// Blocks until the application should exit.
pub(crate) fn run(args: Args) -> iced::Result {
    let initialize = move || -> (App, Task<AppMsg>) {
        let mut app = App::new(
            args.coordinator.clone(),
            args.optimize_touch,
            args.internal_clipboard,
        );

        match Config::load_from_path(util::config_path()) {
            Ok(Some(config)) => app.load_config(config),
            Ok(None) => {
                // Save initially
                app.save_config_to_path();
            }
            Err(error) => {
                error!(?error, "Loading configuration from file");
                app.errors.push(ErrorReport {
                    criticality: ErrorCriticality::NonCritical,
                    short: fl!("error-app-config-load"),
                    detailed: format!("{error:?}"),
                })
            }
        }

        (app, Task::none())
    };

    iced::application(initialize, App::update, views::view_app)
        .title(App::title)
        .settings(iced::Settings {
            default_font: iced::Font::with_name("Cantarell"),
            ..Default::default()
        })
        .window(window::Settings {
            min_size: Some(Size::new(600., 400.)),
            ..Default::default()
        })
        .subscription(App::subscription)
        // Font loading must come *after* initializing settings
        .font(include_bytes!("../data/fonts/Cantarell-Bold.ttf").as_slice())
        .font(include_bytes!("../data/fonts/Cantarell-BoldItalic.ttf").as_slice())
        .font(include_bytes!("../data/fonts/Cantarell-Italic.ttf").as_slice())
        .font(include_bytes!("../data/fonts/Cantarell-Regular.ttf").as_slice())
        .font(include_bytes!("../data/fonts/Inconsolata-VariableFont_wdth_wght.ttf").as_slice())
        .font(include_bytes!("../data/fonts/NotoEmoji-VariableFont_wght.ttf").as_slice())
        .font(BOOTSTRAP_FONT_BYTES)
        //.theme(|_| Theme::Light)
        .antialiasing(true)
        .exit_on_close_request(false)
        .run()
}

/// The current application UI state.
///
/// Note that this is separate from the connection state.
/// It is derived and dicated by the events emitted by the connection subscription,
/// which is responsible for attempting to connect and holding the grpc connection.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub(crate) enum AppState {
    NotConnected(AppNotConnected),
    Connecting { address: String },
    Connected(AppConnected),
}

/// Different modals that can be displayed in the UI.
///
/// Because this is an enum, only a single modal can be displayed at once.
/// This avoids usability challenges/issues that arise when using nested modals.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Default)]
pub(crate) enum Modal {
    #[default]
    None,
    Settings,
    PlaceDetails {
        place_name: String,
    },
    Confirmation {
        msg: String,
        confirm: AppMsg,
    },
}

/// The criticality of of an [ErrorReport].
///
/// Will be used by the UI to use different elements/accents
/// behave differently depending on the criticality.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ErrorCriticality {
    NonCritical,
    Critical,
}

/// An error report intended to be displayed by UI elements.
#[derive(Debug, Clone)]
pub(crate) struct ErrorReport {
    /// The error criticality.
    pub(crate) criticality: ErrorCriticality,
    /// A short string telling users what the issue is in a short and concise way.
    ///
    /// Should be translated.
    pub(crate) short: String,
    /// More verbose message containing details about the error.
    ///
    /// Does not need to be translated, english strings should be fine here.
    ///
    /// Often is just the string representation of a emitted error implementing [std::error::Error]
    pub(crate) detailed: String,
}

/// Holds the entire app state
pub(crate) struct App {
    /// The state that is dependent on the status of the connection.
    pub(crate) state: AppState,
    /// The current displayed modal ([Modal] has variant [Modal::None] when no modal should be displayed).
    pub(crate) modal: Modal,
    /// Optimize the UI for touch input.
    pub(crate) optimize_touch: bool,
    /// App clipboard. Needs to be held for the entire duration of the process.
    pub(crate) clipboard: Option<Clipboard>,
    /// Determines if a internal clipboard implementation should be used instead of delegating copy/pasting
    /// to the system clipboard.
    ///
    /// Useful when the host does not implement a clipboard (e.g. when running on a kiosk wayland compositor like cage).
    pub(crate) internal_clipboard: bool,
    /// The data of the internal clipboard.
    ///
    /// Only used when `internal_clipboard` is set to `true`.
    pub(crate) internal_clipboard_buf: String,
    /// The current app language.
    ///
    /// Whenever the language is changed, the [i18n::change_language] routine is called.
    pub(crate) language: AppLanguage,
    /// The sender that sends messages to the connection subscription.
    pub(crate) connection_sender: Option<ConnectionSender>,
    /// All current reported errors.
    pub(crate) errors: Vec<ErrorReport>,
    /// The current set python virtual environment directory.
    ///
    /// Used when executing scripts in the UI scripts tab.
    pub(crate) venv_dir: PathBuf,
    /// The current set scripts directory.
    ///
    /// Used for listing scripts in the UI scripts tab.
    pub(crate) scripts_dir: PathBuf,
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("state", &self.state)
            .field("modal", &self.modal)
            .field("optimize_touch", &self.optimize_touch)
            .field("clipboard", &".. no debug impl ..")
            .field("internal_clipboard", &self.internal_clipboard)
            .field("internal_clipboard_buf", &self.internal_clipboard_buf)
            .field("language", &self.language)
            .field("connection_sender", &self.connection_sender)
            .field("errors", &self.errors)
            .field("venv_dir", &self.venv_dir)
            .field("scripts_dir", &self.scripts_dir)
            .finish()
    }
}

impl App {
    /// Create a new application with options:
    /// - the supplied labgrid coordinator address.
    ///   will fill the address field of the UI in the not-connected state,
    ///   but will not connect automatically on it's own.
    /// - whether the UI should be optimized for touch input.
    /// - whether the internal clipboard implementation should be used.
    fn new(
        coordinator_address: Option<String>,
        optimize_touch: bool,
        internal_clipboard: bool,
    ) -> Self {
        debug!(?coordinator_address, ?optimize_touch, "New app");
        if let Err(err) = util::ensure_app_default_dirs() {
            error!(?err, "Ensure existance of app default dirs");
        };
        let clipboard = if internal_clipboard {
            None
        } else {
            Clipboard::new().ok()
        };

        Self {
            state: AppState::NotConnected(AppNotConnected {
                input_address: coordinator_address.unwrap_or_default(),
            }),
            language: AppLanguage::try_from(i18n::current_language())
                .expect("Loaded language is not a variant of 'AppLanguage'"),
            modal: Modal::None,
            optimize_touch,
            clipboard,
            internal_clipboard,
            internal_clipboard_buf: String::default(),
            connection_sender: None,
            errors: Vec::default(),
            venv_dir: util::default_venv_dir(),
            scripts_dir: util::default_scripts_dir(),
        }
    }

    /// Returns the (translated) application title.
    fn title(&self) -> String {
        fl!("app-title")
    }

    /// Returns all joined subscription.
    fn subscription(&self) -> Subscription<AppMsg> {
        let subscriptions = [
            Subscription::run(connection::kickoff).map(AppMsg::ConnectionEvent),
            Subscription::run(config::periodic_save_subscription),
            window::close_requests().map(AppMsg::CloseWindow),
        ];
        Subscription::batch(subscriptions)
    }

    /// Handle received app messages through iced's message passing.
    fn update(&mut self, msg: AppMsg) -> Task<AppMsg> {
        debug!(?msg, "App UI update");

        let (new_state, task): (Option<AppState>, Task<AppMsg>) = match msg {
            AppMsg::None => (None, Task::none()),
            AppMsg::ChangeLanguage(language) => {
                if self.language != language {
                    match i18n::change_language(language.into()) {
                        Ok(_) => {
                            self.language = language;
                        }
                        Err(error) => error!(?error, ?language, "Change language"),
                    }
                }
                (None, Task::none())
            }
            AppMsg::OptimizeTouch(optimize_touch) => {
                self.optimize_touch = optimize_touch;
                (None, Task::none())
            }
            AppMsg::ClipboardCopy(content) => {
                if let Err(e) = set_clipboard_text(
                    &mut self.clipboard,
                    self.internal_clipboard,
                    &mut self.internal_clipboard_buf,
                    content,
                ) {
                    error!("Set clipboard content, Err: {e:?}");
                    self.errors.push(ErrorReport {
                        criticality: ErrorCriticality::NonCritical,
                        short: "Set clipboard content".to_string(),
                        detailed: format!("{e:?}"),
                    });
                }
                (None, Task::none())
            }
            AppMsg::SaveConfig => {
                self.save_config_to_path();
                (None, Task::none())
            }
            AppMsg::CloseLatestWindow => {
                self.save_config_to_path();
                (None, window::latest().and_then(window::close))
            }
            AppMsg::CloseWindow(id) => {
                self.save_config_to_path();
                (None, window::close(id))
            }
            AppMsg::ShowModal(modal) => {
                self.modal = *modal;
                (None, Task::none())
            }
            AppMsg::HideModal => {
                self.modal = Modal::None;
                (None, Task::none())
            }
            AppMsg::WithHideModal(msg) => {
                // Recursing like that is not the most awesome pattern, but eh it works
                self.modal = Modal::None;
                (None, self.update(*msg))
            }
            AppMsg::DismissError => {
                self.errors.pop();
                (None, Task::none())
            }
            AppMsg::ChangeVenvDir { dir } => {
                match scripts::validate_venv_dir(&dir) {
                    Ok(()) => self.venv_dir = dir,
                    Err(err) => {
                        error!(
                            ?err,
                            "Validation while attempting to change labgrid venv dir failed"
                        );
                        self.errors.push(ErrorReport {
                            criticality: ErrorCriticality::NonCritical,
                            short: fl!("error-invalid-path"),
                            detailed: format!("Invalid labgrid venv path: '{}'", dir.display()),
                        });
                    }
                }
                (None, Task::none())
            }
            AppMsg::ChangeScriptsDir { dir } => {
                match Scripts::from_dir(dir.clone()) {
                    Ok(scripts) => {
                        self.scripts_dir = scripts.dir();
                        if let AppState::Connected(connected) = &mut self.state {
                            connected.scripts = scripts;
                        }
                    }
                    Err(err) => {
                        error!(
                            ?err,
                            "Validation while attempting to change scripts dir failed"
                        );
                        self.errors.push(ErrorReport {
                            criticality: ErrorCriticality::NonCritical,
                            short: fl!("error-invalid-path"),
                            detailed: format!("Invalid scripts directory : '{}'", dir.display()),
                        });
                    }
                }
                (None, Task::none())
            }
            AppMsg::ConnectionMsg(msg) => {
                if let Some(sender) = &mut self.connection_sender {
                    sender.send(msg);
                }
                (None, Task::none())
            }
            AppMsg::ConnectionEvent(ConnectionEvent::ReceiveReady(sender)) => {
                self.connection_sender = Some(sender);
                (None, Task::none())
            }
            AppMsg::ConnectionEvent(ConnectionEvent::Disconnected { error }) => {
                if let Some(error) = error {
                    error!(?error, "Disconnect with error");
                    self.errors.push(error);
                }
                debug!("Disconnected");
                let address = self.coordinator_address();
                let new_state = AppState::NotConnected(AppNotConnected::with_address(address));
                (Some(new_state), Task::none())
            }
            AppMsg::ConnectionEvent(ConnectionEvent::NonCriticalError { error }) => {
                warn!(?error, "Non-critical connection error");
                self.errors.push(error);
                (None, Task::none())
            }
            AppMsg::ConnectionEvent(ConnectionEvent::Place(place)) => {
                debug!(?place, "Refreshing place data");
                if let AppState::Connected(connected) = &mut self.state {
                    connected.place_add_replace(place);
                }
                (None, Task::none())
            }
            AppMsg::ConnectionEvent(ConnectionEvent::DeletePlace(name)) => {
                debug!("Deleting place");
                if let AppState::Connected(connected) = &mut self.state {
                    connected.delete_place(name);
                }
                (None, Task::none())
            }
            AppMsg::ConnectionEvent(ConnectionEvent::Places(places)) => {
                debug!("Refreshing places");
                if let AppState::Connected(connected) = &mut self.state {
                    connected.places = places
                        .into_iter()
                        .map(|p| (p, PlaceUi::default()))
                        .collect();
                    connected.sort_places();
                }
                (None, Task::none())
            }
            AppMsg::ConnectionEvent(ConnectionEvent::Reservations(reservations)) => {
                debug!("Refreshing reservations");
                if let AppState::Connected(connected) = &mut self.state {
                    connected.reservations = reservations;
                    connected.sort_reservations();
                }
                (None, Task::none())
            }
            AppMsg::ConnectionEvent(ConnectionEvent::Resource(resource)) => {
                debug!("Add/refreshing resource");
                if let AppState::Connected(connected) = &mut self.state {
                    connected.resource_add_replace(resource);
                }
                (None, Task::none())
            }
            AppMsg::ConnectionEvent(ConnectionEvent::DeleteResource(path)) => {
                debug!("Deleting resource");
                if let AppState::Connected(connected) = &mut self.state {
                    connected.remove_resource(path);
                }
                (None, Task::none())
            }
            AppMsg::ConnectionEvent(ConnectionEvent::Connected { address }) => {
                let new_state =
                    AppState::Connected(AppConnected::new(address, self.scripts_dir.clone()));
                // For some reason reservations are not part of the client syncing..
                send_connection_msg(&mut self.connection_sender, ConnectionMsg::GetReservations);
                (Some(new_state), Task::none())
            }
            AppMsg::NotConnected(msg) => {
                if let AppState::NotConnected(not_connected) = &mut self.state {
                    not_connected.update(msg, &mut self.connection_sender)
                } else {
                    (None, Task::none())
                }
            }
            AppMsg::Connected(msg) => {
                if let AppState::Connected(connected) = &mut self.state {
                    connected.update(
                        msg,
                        &mut self.connection_sender,
                        &mut self.clipboard,
                        self.internal_clipboard,
                        &mut self.internal_clipboard_buf,
                        &mut self.errors,
                        &self.venv_dir,
                    )
                } else {
                    (None, Task::none())
                }
            }
        };
        if let Some(new_state) = new_state {
            self.state = new_state;
        }

        task
    }

    pub(crate) fn load_config(&mut self, config: Config) {
        self.language = config.language;
        self.optimize_touch = config.optimize_touch;
        self.venv_dir = config.venv_dir;
        self.scripts_dir = config.scripts_dir;
    }

    pub(crate) fn extract_config(&self) -> Config {
        let coordinator_address = if let AppState::Connected(connected) = &self.state {
            connected.address.clone()
        } else {
            String::default()
        };
        Config {
            coordinator_address,
            language: self.language,
            optimize_touch: self.optimize_touch,
            venv_dir: self.venv_dir.clone(),
            scripts_dir: self.scripts_dir.clone(),
        }
    }

    /// Saves the current application configuration to the FS.
    ///
    /// If it fails, an error is reported in the UI and as event.
    pub(crate) fn save_config_to_path(&mut self) {
        let config = self.extract_config();
        if let Err(error) = config.save_to_path(util::config_path()) {
            error!(?error, "Saving configuration to file");
            self.errors.push(ErrorReport {
                criticality: ErrorCriticality::Critical,
                short: fl!("error-app-config-save"),
                detailed: format!("{error:?}"),
            });
        }
    }

    /// Returns the coordinator address either from the text input or active connection depending on the app state.
    ///
    /// When not connnected, returns the state of the address field,
    /// when connected the address of the current gRPC connection.
    pub(crate) fn coordinator_address(&self) -> String {
        match &self.state {
            AppState::NotConnected(not_connected) => not_connected.input_address.clone(),
            AppState::Connecting { address } => address.clone(),
            AppState::Connected(connected) => connected.address.clone(),
        }
    }
}

/// Get the clipboard text.
///
/// Retrieves from the system clipboard if `internal_clipboard` is set false,
/// or from `clipboard` if set true.
fn clipboard_text(
    clipboard: &mut Option<Clipboard>,
    internal_clipboard: bool,
    internal_clipboard_buf: &str,
) -> anyhow::Result<String> {
    debug!("Get clipboard text");

    if let Some(clipboard) = clipboard {
        clipboard.get_text().context("Get clipboard text")
    } else if internal_clipboard {
        Ok(internal_clipboard_buf.to_owned())
    } else {
        Ok(String::default())
    }
}

/// Set the clipboard.
///
/// Set the system clipboard text if `internal_clipboard` is set to false,
/// or `clipboard` if set true.
fn set_clipboard_text(
    clipboard: &mut Option<Clipboard>,
    internal_clipboard: bool,
    internal_clipboard_buf: &mut String,
    text: String,
) -> anyhow::Result<()> {
    debug!("Set clipboard text");
    if let Some(clipboard) = clipboard {
        clipboard.set_text(text).context("Set clipboard text")
    } else if internal_clipboard {
        *internal_clipboard_buf = text;
        Ok(())
    } else {
        Ok(())
    }
}

/// Holds app state when in not-connected state.
#[derive(Debug)]
pub(crate) struct AppNotConnected {
    pub(crate) input_address: String,
}

impl AppNotConnected {
    #[allow(unused)]
    fn new() -> Self {
        Self {
            input_address: String::default(),
        }
    }

    /// New not-connected app state with the supplied coordinator address.
    fn with_address(coordinator_address: String) -> Self {
        Self {
            input_address: coordinator_address,
        }
    }

    /// Handle received not-connected messages through delegation by the top-level app message handler.
    ///
    /// Returns `(<new-app-state>, <app-task>)`.
    ///
    /// When `<new-app-state>` is [Option::Some], the app will transition into the new state
    /// by the top-level app message handler.
    fn update(
        &mut self,
        msg: NotConnectedMsg,
        connection_sender: &mut Option<ConnectionSender>,
    ) -> (Option<AppState>, Task<AppMsg>) {
        match msg {
            NotConnectedMsg::Connect => {
                let Some(sender) = connection_sender else {
                    warn!("Connection not yet ready");
                    return (None, Task::none());
                };
                debug!(
                    address = self.input_address,
                    "Attempting to connect to gRPC server"
                );
                sender.send(ConnectionMsg::Connect {
                    address: self.input_address.clone(),
                });
                let new_state = AppState::Connecting {
                    address: self.input_address.clone(),
                };
                (Some(new_state), Task::none())
            }
            NotConnectedMsg::UpdateInputAddress(input_address) => {
                self.input_address = input_address;
                (None, Task::none())
            }
        }
    }
}

/// Holds additional data needed to display and interact with the widgets presenting a single resource.
#[derive(Debug, Clone)]
pub(crate) struct ResourceUi {
    pub(crate) show_details: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for ResourceUi {
    fn default() -> Self {
        Self {
            show_details: false,
        }
    }
}

/// Holds additional data needed to display and interact with the widgets presenting a single place.
#[derive(Debug, Clone)]
pub(crate) struct PlaceUi {
    pub(crate) add_tag_text: Option<(String, String)>,
}

#[allow(clippy::derivable_impls)]
impl Default for PlaceUi {
    fn default() -> Self {
        Self { add_tag_text: None }
    }
}

#[derive(Debug)]
pub(crate) struct AppConnected {
    pub(crate) address: String,
    pub(crate) active_tab: TabId,
    pub(crate) places: Vec<(Place, PlaceUi)>,
    // TODO: more efficient/better fitting data structure than a Vec, possibly HashMap?
    pub(crate) reservations: Vec<Reservation>,
    // TODO: more efficient/better fitting data structure than a Vec, possibly HashMap?
    pub(crate) resources: Vec<(Resource, ResourceUi)>,
    pub(crate) resources_only_show_available: bool,
    pub(crate) add_place_text: String,
    pub(crate) add_place_match_text: String,
    pub(crate) scripts: Scripts,
    pub(crate) script_out: String,
    pub(crate) script_status: scripts::ScriptStatus,
    pub(crate) script_show_output: bool,
}

impl AppConnected {
    /// Create a new connected app state.
    fn new(address: String, scripts_dir: PathBuf) -> Self {
        Self {
            address,
            active_tab: TabId::default(),
            places: Vec::default(),
            reservations: Vec::default(),
            resources: Vec::default(),
            resources_only_show_available: true,
            add_place_text: String::default(),
            add_place_match_text: String::default(),
            // First attempt to discover scripts in default dir,
            // if it fails fall back to default (no scripts enumerated)
            scripts: Scripts::from_dir(scripts_dir).unwrap_or_default(),
            script_status: scripts::ScriptStatus::None,
            script_out: String::default(),
            script_show_output: false,
        }
    }

    /// Handle received not-connected messages through delegation by the top-level app message handler.
    ///
    /// Returns `(<new-app-state>, <app-task>)`.
    ///
    /// When `<new-app-state>` is [Option::Some], the app will transition into the hew state
    /// by the top-level app message handler.
    #[allow(clippy::too_many_arguments)]
    fn update(
        &mut self,
        msg: ConnectedMsg,
        connection_sender: &mut Option<ConnectionSender>,
        clipboard: &mut Option<Clipboard>,
        internal_clipboard: bool,
        internal_clipboard_buf: &mut str,
        errors: &mut Vec<ErrorReport>,
        venv_dir: &Path,
    ) -> (Option<AppState>, Task<AppMsg>) {
        match msg {
            ConnectedMsg::Disconnect => {
                send_connection_msg(connection_sender, ConnectionMsg::Disconnect);
                (None, Task::none())
            }
            ConnectedMsg::Refresh => {
                send_connection_msg(connection_sender, ConnectionMsg::Sync);
                // For some reason reservations are not part of the client syncing..
                send_connection_msg(connection_sender, ConnectionMsg::GetReservations);
                (None, Task::none())
            }
            ConnectedMsg::TabSelected(tab) => {
                tracing::debug!("New tab selected {tab:?}");
                self.active_tab = tab;
                (None, Task::none())
            }
            ConnectedMsg::UpdateAddPlaceName(text) => {
                self.add_place_text = text;
                (None, Task::none())
            }
            ConnectedMsg::ClipboardPasteAddPlaceName => {
                match clipboard_text(clipboard, internal_clipboard, internal_clipboard_buf) {
                    Ok(text) => self.add_place_text = text,
                    Err(e) => {
                        error!("Paste clipboard into add place text field, Err: {e:?}");
                        errors.push(ErrorReport {
                            criticality: ErrorCriticality::NonCritical,
                            short: "Paste clipboard into add place text field".to_string(),
                            detailed: format!("{e:?}"),
                        });
                    }
                }
                (None, Task::none())
            }
            ConnectedMsg::ShowResourceDetails(path) => {
                self.resource_set_show_details(path, true);
                self.add_place_match_text.clear();
                (None, Task::none())
            }
            ConnectedMsg::ResourcesOnlyShowAvailable(show) => {
                self.resources_only_show_available = show;
                (None, Task::none())
            }
            ConnectedMsg::HideResourceDetails(path) => {
                self.resource_set_show_details(path, false);
                (None, Task::none())
            }
            ConnectedMsg::UpdateAddPlaceMatchPattern(text) => {
                self.add_place_match_text = text;
                (None, Task::none())
            }
            ConnectedMsg::ClipboardPasteAddPlaceMatchPattern => {
                match clipboard_text(clipboard, internal_clipboard, internal_clipboard_buf) {
                    Ok(text) => self.add_place_match_text = text,
                    Err(e) => {
                        error!("Paste clipboard into add place match text field, Err: {e:?}");
                        errors.push(ErrorReport {
                            criticality: ErrorCriticality::NonCritical,
                            short: "Paste clipboard into add place match text field".to_string(),
                            detailed: format!("{e:?}"),
                        });
                    }
                }
                (None, Task::none())
            }
            ConnectedMsg::ShowAddPlaceTag { place_name } => {
                if let Some((_, ui)) = self.place_by_name_mut(&place_name) {
                    ui.add_tag_text = Some((String::default(), String::default()));
                }
                (None, Task::none())
            }
            ConnectedMsg::CloseAddPlaceTag { place_name } => {
                if let Some((_, ui)) = self.place_by_name_mut(&place_name) {
                    ui.add_tag_text = None;
                }
                (None, Task::none())
            }
            ConnectedMsg::UpdateAddPlaceTagText { place_name, text } => {
                if let Some((_, ui)) = self.place_by_name_mut(&place_name) {
                    ui.add_tag_text = Some((
                        text,
                        ui.add_tag_text.take().map(|t| t.1).unwrap_or_default(),
                    ));
                }
                (None, Task::none())
            }
            ConnectedMsg::UpdateAddPlaceTagValueText { place_name, text } => {
                if let Some((_, ui)) = self.place_by_name_mut(&place_name) {
                    ui.add_tag_text = Some((
                        ui.add_tag_text.take().map(|t| t.0).unwrap_or_default(),
                        text,
                    ));
                }
                (None, Task::none())
            }
            ConnectedMsg::ClearAddPlaceTagText { place_name } => {
                if let Some((_, ui)) = self.place_by_name_mut(&place_name) {
                    ui.add_tag_text = Some((String::default(), String::default()));
                }
                (None, Task::none())
            }
            ConnectedMsg::OpenChangeScriptsDirDialog { initial_dir } => {
                let task = Task::perform(
                    async move {
                        let res = rfd::AsyncFileDialog::new()
                            .add_filter(fl!("file-dialog-filter-python-scripts-label"), &["py"])
                            .set_directory(initial_dir)
                            .pick_folder()
                            .await;
                        res.map(|f| f.path().to_owned())
                    },
                    |res| {
                        if let Some(dir) = res {
                            AppMsg::ChangeScriptsDir { dir }
                        } else {
                            AppMsg::None
                        }
                    },
                );
                (None, task)
            }
            ConnectedMsg::OpenChangeVenvDirFileDialog { initial_dir } => {
                let task = Task::perform(
                    async move {
                        let res = rfd::AsyncFileDialog::new()
                            .set_directory(initial_dir)
                            .pick_folder()
                            .await;
                        res.map(|f| f.path().to_owned())
                    },
                    |res| {
                        if let Some(dir) = res {
                            AppMsg::ChangeVenvDir { dir }
                        } else {
                            AppMsg::None
                        }
                    },
                );
                (None, task)
            }
            ConnectedMsg::RescanScriptsDir => {
                if let Err(err) = self.scripts.rescan() {
                    error!(?err, "Scripts dir rescan failed");
                    errors.push(ErrorReport {
                        criticality: ErrorCriticality::NonCritical,
                        short: fl!("scripts-dir-rescan-failed-error"),
                        detailed: format!(
                            "Scripts dir : {}, Err : {err:?}",
                            self.scripts.dir().display()
                        ),
                    })
                }

                (None, Task::none())
            }
            ConnectedMsg::ExecuteScript { script } => {
                let venv_dir = venv_dir.to_owned();
                let env = self.scripts.env.clone();
                let script_c = script.clone();
                let script_c2 = script.clone();
                self.script_out.clear();
                self.script_out += &format!("### Executing script ###\nEnv:\n{env}");
                let (task, handle) = Task::abortable(Task::perform(
                    async move { script.execute(&venv_dir, &env).await },
                    move |out| match out {
                        Ok((exit_code, stdout, stderr)) => {
                            AppMsg::Connected(ConnectedMsg::ScriptFinished {
                                script: script_c.clone(),
                                exit_code,
                                stdout,
                                stderr,
                            })
                        }
                        Err(err) => AppMsg::Connected(ConnectedMsg::ScriptExecutionFailed {
                            script: script_c.clone(),
                            err: format!("{err:?}"),
                        }),
                    },
                ));
                self.script_status = ScriptStatus::Running {
                    script: script_c2,
                    handle: handle.abort_on_drop(),
                };
                (None, task)
            }
            ConnectedMsg::AbortScript => {
                // Handle aborts script task on drop
                self.script_status = ScriptStatus::None;
                self.script_out.clear();
                (None, Task::none())
            }
            ConnectedMsg::ScriptFinished {
                script,
                exit_code,
                stdout,
                stderr,
            } => {
                self.script_status = ScriptStatus::Finished { script, exit_code };
                self.script_out +=
                    &format!("### Script Stdout ###\n{stdout}\n### Script Stderr ###\n{stderr}");
                (None, Task::none())
            }
            ConnectedMsg::ScriptExecutionFailed { script, err } => {
                self.script_status = ScriptStatus::None;
                self.script_out.clear();
                errors.push(ErrorReport {
                    criticality: ErrorCriticality::Critical,
                    short: fl!("script-failed-msg"),
                    detailed: format!("Script: '{}', Err: {err}", script.path().display()),
                });
                (None, Task::none())
            }
            ConnectedMsg::ScriptsEnvUpdate { entry, value } => {
                self.scripts.env.insert(entry, value);
                (None, Task::none())
            }
            ConnectedMsg::ScriptsEnvClear { entry } => {
                self.scripts.env.remove(&entry);
                (None, Task::none())
            }
            ConnectedMsg::ScriptsEnvOpenLgEnvFileDialog { initial_file } => {
                let task = Task::perform(
                    async move {
                        let mut dialog = rfd::AsyncFileDialog::new();
                        if let Some(parent_dir) = initial_file.parent() {
                            dialog = dialog.set_directory(parent_dir);
                        };
                        let res = dialog
                            .add_filter("YAML", &["yml", "yaml"])
                            .pick_file()
                            .await;
                        res.map(|f| f.path().to_owned())
                    },
                    |res| {
                        if let Some(file) = res {
                            AppMsg::Connected(ConnectedMsg::ScriptsEnvUpdate {
                                entry: EnvEntry::LgEnv,
                                value: file.to_string_lossy().to_string(),
                            })
                        } else {
                            AppMsg::None
                        }
                    },
                );
                (None, task)
            }
            ConnectedMsg::ScriptOutShow => {
                self.script_show_output = true;
                (None, Task::none())
            }
            ConnectedMsg::ScriptOutHide => {
                self.script_show_output = false;
                (None, Task::none())
            }
            ConnectedMsg::ScriptOutClear => {
                self.script_out.clear();
                (None, Task::none())
            }
        }
    }

    /// Returns a immutable reference to the place whose name matches with the supplied name.
    pub(crate) fn place_by_name<'a>(&'a self, name: &'a str) -> Option<&'a (Place, PlaceUi)> {
        self.places.iter().find(|(p, _)| p.name == name)
    }

    /// Returns a mutable reference to the place whose name matches with the supplied name.
    pub(crate) fn place_by_name_mut<'a>(
        &'a mut self,
        name: &'a str,
    ) -> Option<&'a mut (Place, PlaceUi)> {
        self.places.iter_mut().find(|(p, _)| p.name == name)
    }

    /// Sort the places into human-expected order for display by the UI.
    pub(crate) fn sort_places(&mut self) {
        self.places
            .sort_by(|(first, _), (second, _)| numeric_sort::cmp(&first.name, &second.name));
        self.places.iter_mut().for_each(|(p, _)| {
            p.acquired_resources
                .sort_by(|first, second| numeric_sort::cmp(first, second))
        });
        self.places
            .iter_mut()
            .for_each(|(p, _)| p.matches.sort_by(|first, second| first.numeric_cmp(second)));
    }

    /// Sort the reservations into human-expected order for display by the UI.
    pub(crate) fn sort_reservations(&mut self) {
        self.reservations
            .sort_by(|first, second| numeric_sort::cmp(&first.owner, &second.owner));
    }

    /// Sort the resources into human-expected order for display by the UI.
    pub(crate) fn sort_resources(&mut self) {
        self.resources
            .sort_by(|(first, _), (second, _)| first.path.numeric_cmp(&second.path));
    }

    /// Adds or replaces a resource.
    ///
    /// When the resource path matches the resource is replaced.
    /// Otherwise it will be inserted.
    ///
    /// Sorts the resources after insertion/replacement.
    pub(crate) fn resource_add_replace(&mut self, resource: Resource) {
        if let Some((found, _)) = self
            .resources
            .iter_mut()
            .find(|(r, _)| r.path == resource.path)
        {
            *found = resource;
        } else {
            self.resources.push((resource, ResourceUi::default()));
        }
        self.sort_resources();
    }

    /// Remove a specific resource with the supplied path.
    ///
    /// Returns [Option::Some} if the resource was found and removed, [Option::None]
    /// if it was not present (and therefore could not be removed).
    pub(crate) fn remove_resource(&mut self, path: types::Path) -> Option<(Resource, ResourceUi)> {
        let (i, _) = self
            .resources
            .iter()
            .enumerate()
            .find(|(_, (r, _))| r.path == path)?;
        Some(self.resources.remove(i))
    }

    /// Toggles whether resource details should be shown in the UI.
    pub(crate) fn resource_set_show_details(&mut self, path: types::Path, show_details: bool) {
        if let Some((_, ui)) = self.resources.iter_mut().find(|(r, _)| r.path == path) {
            ui.show_details = show_details;
        } else {
            error!(
                ?path,
                "Attempted to show resource details that could not be found"
            );
        };
    }

    /// Adds or replaces a place.
    ///
    /// When the place name matches, it is replaced,
    /// otherwise the supplied place is inserted.
    ///
    /// Sorts the places after insertion/replacement.
    pub(crate) fn place_add_replace(&mut self, place: Place) {
        if let Some(found) = self.places.iter_mut().find(|(p, _)| p.name == place.name) {
            *found = (place, PlaceUi::default());
        } else {
            self.places.push((place, PlaceUi::default()));
        }
        self.sort_places();
    }

    /// Deletes a place with the supplied name.
    ///
    /// Returns [Option::Some} if the place was found and removed, [Option::None]
    /// if it was not present (and therefore could not be removed).
    pub(crate) fn delete_place(&mut self, name: String) -> Option<Place> {
        let (i, _) = self
            .places
            .iter()
            .enumerate()
            .find(|(_, (p, _))| p.name == name)?;
        Some(self.places.remove(i)).map(|(p, _)| p)
    }
}

/// Send a message to the connection subscription.
fn send_connection_msg(connection_sender: &mut Option<ConnectionSender>, msg: ConnectionMsg) {
    let Some(sender) = connection_sender else {
        warn!("Connection not yet ready.");
        return;
    };
    sender.send(msg);
}
