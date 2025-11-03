// SPDX-FileCopyrightText: 2025 Duagon Germany GmbH
//
// SPDX-License-Identifier: GPL-3.0-or-later

use i18n_embed::fluent::{fluent_language_loader, FluentLanguageLoader};
use i18n_embed::{DesktopLanguageRequester, LanguageLoader};
use once_cell::sync::Lazy;
use tracing::{debug, error};

/// Embeds the localization data.
#[derive(rust_embed::RustEmbed)]
#[folder = "i18n"] // path to the compiled localization resources
struct Localizations;

/// Lazy initialized language loader which holds state about the currently used and fallback languages
/// and the translations for them.
pub(crate) static LOADER: Lazy<FluentLanguageLoader> = Lazy::new(|| {
    let loader = fluent_language_loader!();
    loader
        .load_fallback_language(&Localizations)
        .expect("Loading fallback language");
    let requested_languages = DesktopLanguageRequester::requested_languages();
    debug!(?requested_languages, "Loading initial requested languages");
    if let Err(error) = loader.load_languages(&Localizations, &requested_languages) {
        error!(?error, "Load initial requested language");
    }
    loader
});

/// Convenience macro to access translations without having to specify the language loader.
///
/// Enables compile time checked queries.
///
/// Usage:
/// ```rust
/// fl!("message-id");
/// fl!("message-id", args);
/// fl!("message-id", arg = "value");
/// ```
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::i18n::LOADER, $message_id)
    }};

    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!($crate::i18n::LOADER, $message_id, $($args), *)
    }};
}
pub(crate) use fl;

use anyhow::Context;
use core::fmt::Display;

/// Returns the current active language.
pub(crate) fn current_language() -> i18n_embed::unic_langid::LanguageIdentifier {
    LOADER.current_language()
}

/// Changes the current active language.
pub(crate) fn change_language(
    language: i18n_embed::unic_langid::LanguageIdentifier,
) -> anyhow::Result<()> {
    debug!(?language, "Load new language");
    LOADER
        .load_languages(&Localizations, &[language])
        .context("Load new language")
}

/// Holds all currently supported app languages.
///
/// Must correspond to the presence of files in folder `i18n`.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize,
)]
pub(crate) enum AppLanguage {
    DeCh,
    DeDe,
    #[default]
    EnUs,
    EsEs,
}

impl Display for AppLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppLanguage::DeCh => write!(f, "{}", fl!("lang-de-ch")),
            AppLanguage::DeDe => write!(f, "{}", fl!("lang-de-de")),
            AppLanguage::EnUs => write!(f, "{}", fl!("lang-en-us")),
            AppLanguage::EsEs => write!(f, "{}", fl!("lang-es-es")),
        }
    }
}

impl From<AppLanguage> for i18n_embed::unic_langid::LanguageIdentifier {
    fn from(value: AppLanguage) -> Self {
        match value {
            AppLanguage::DeCh => "de-CH".parse().unwrap(),
            AppLanguage::DeDe => "de-DE".parse().unwrap(),
            AppLanguage::EnUs => "en-US".parse().unwrap(),
            AppLanguage::EsEs => "es-ES".parse().unwrap(),
        }
    }
}

impl TryFrom<i18n_embed::unic_langid::LanguageIdentifier> for AppLanguage {
    type Error = anyhow::Error;

    fn try_from(value: i18n_embed::unic_langid::LanguageIdentifier) -> Result<Self, Self::Error> {
        match (
            value.language.as_str().to_lowercase().as_str(),
            value.region.map(|s| s.as_str().to_lowercase()).as_deref(),
        ) {
            ("de", Some("ch")) => Ok(Self::DeCh),
            ("de", Some("de")) | ("de", None) => Ok(Self::DeDe),
            ("en", Some("us")) | ("en", None) => Ok(Self::EnUs),
            ("es", Some("es")) | ("es", None) => Ok(Self::EsEs),
            (lang, region) => Err(anyhow::anyhow!(
                "Conversion to AppLanguage failed, unsupported language '{lang}-{region:?}'"
            )),
        }
    }
}

impl AppLanguage {
    /// All currently available languages as a slice.
    pub(crate) const LANGS_AVAILABLE: &'static [Self] =
        &[Self::DeCh, Self::DeDe, Self::EnUs, Self::EsEs];
}
