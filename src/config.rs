//! Persistent settings, stored via `cosmic_config`.
//!
//! Lives at `~/.config/cosmic/io.github.atayozcan.CosmicCaffeine/v1/<field>`,
//! one RON-encoded file per field. cosmic_config is the COSMIC-native
//! config story (used by all upstream applets), gives us cross-process
//! live reload via inotify for free, and lets a future
//! `cosmic-settings` integration discover the schema without changes
//! here.

use cosmic_config::{Config, CosmicConfigEntry};
// Derive macro lives in the sibling `cosmic-config-derive` crate, re-exported
// by `cosmic_config` as a sub-module. Macros and traits inhabit different
// namespaces, so this second `use` doesn't shadow the trait import above.
use cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use serde::{Deserialize, Serialize};

use crate::APP_ID;

pub const CONFIG_VERSION: u64 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, CosmicConfigEntry)]
#[version = 1]
pub struct CaffeineConfig {
    /// 0 = inhibit indefinitely (until toggled off).
    pub default_minutes: u32,
    pub inhibit_idle: bool,
    pub inhibit_sleep: bool,
    pub notify_on_toggle: bool,
}

impl Default for CaffeineConfig {
    fn default() -> Self {
        Self {
            default_minutes: 0,
            inhibit_idle: true,
            inhibit_sleep: true,
            notify_on_toggle: false,
        }
    }
}

/// Open a handler for the cosmic_config namespace this app owns.
/// Returns an error only when the cosmic_config layer can't access the
/// XDG config dir at all — first-run with no existing files is fine.
pub fn handler() -> Result<Config, cosmic_config::Error> {
    Config::new(APP_ID, CONFIG_VERSION)
}

/// Read the config, falling back to defaults on partial-read errors.
/// Errors are logged and ignored — a corrupt single-field file
/// shouldn't keep the daemon from running.
pub fn load() -> CaffeineConfig {
    let Ok(h) = handler() else {
        return CaffeineConfig::default();
    };
    match CaffeineConfig::get_entry(&h) {
        Ok(cfg) => cfg,
        Err((errs, cfg)) => {
            for e in errs {
                eprintln!("cosmic-caffeine: config: {e}");
            }
            cfg
        }
    }
}

pub fn save(cfg: &CaffeineConfig) -> Result<(), cosmic_config::Error> {
    let h = handler()?;
    cfg.write_entry(&h)
}
