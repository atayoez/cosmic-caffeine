//! Fluent-driven i18n setup, mirroring the upstream pop-os/cosmic-applets
//! pattern. The `fl!` macro is the user-facing handle: `fl!("foo")` for
//! parameter-free messages, `fl!("foo", arg = "x")` for parameterized.

use i18n_embed::fluent::{fluent_language_loader, FluentLanguageLoader};
use i18n_embed::{DefaultLocalizer, DesktopLanguageRequester, LanguageLoader, Localizer};
use rust_embed::RustEmbed;
use std::sync::LazyLock;

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

pub static LANGUAGE_LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();
    loader
        .load_fallback_language(&Localizations)
        .expect("loading fallback language");
    loader
});

pub fn localizer() -> Box<dyn Localizer> {
    Box::new(DefaultLocalizer::new(&*LANGUAGE_LOADER, &Localizations))
}

/// Apply the desktop's requested languages to the loader. Idempotent;
/// safe to call from both the daemon and settings binaries.
pub fn localize() {
    let localizer = localizer();
    let requested = DesktopLanguageRequester::requested_languages();
    if let Err(e) = localizer.select(&requested) {
        eprintln!("cosmic-caffeine: i18n select: {e}");
    }
}

/// Translate a Fluent message id, optionally with named arguments.
///
/// ```ignore
/// fl!("menu-quit");
/// fl!("menu-on-for", minutes = 5);
/// ```
#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::localize::LANGUAGE_LOADER, $message_id)
    }};
    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!($crate::localize::LANGUAGE_LOADER, $message_id, $($args),*)
    }};
}
