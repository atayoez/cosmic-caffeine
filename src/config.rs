use serde::{Deserialize, Serialize};
use std::path::Path;

pub fn default_default_minutes() -> u32 {
    0
}
pub fn default_inhibit_idle() -> bool {
    true
}
pub fn default_inhibit_sleep() -> bool {
    true
}
pub fn default_notify_on_toggle() -> bool {
    false
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    /// 0 = indefinite (until toggled off).
    #[serde(default = "default_default_minutes")]
    pub default_minutes: u32,
    #[serde(default = "default_inhibit_idle")]
    pub inhibit_idle: bool,
    #[serde(default = "default_inhibit_sleep")]
    pub inhibit_sleep: bool,
    #[serde(default = "default_notify_on_toggle")]
    pub notify_on_toggle: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_minutes: default_default_minutes(),
            inhibit_idle: default_inhibit_idle(),
            inhibit_sleep: default_inhibit_sleep(),
            notify_on_toggle: default_notify_on_toggle(),
        }
    }
}

pub fn read(path: &Path) -> Result<Config, String> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let s = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    toml::from_str(&s).map_err(|e| format!("parse {}: {e}", path.display()))
}

pub fn write(path: &Path, cfg: &Config) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = String::new();
    out.push_str("# cosmic-caffeine config\n");
    out.push_str("# default_minutes = 0 means inhibit indefinitely (until toggled off).\n\n");
    out.push_str(&format!("default_minutes  = {}\n", cfg.default_minutes));
    out.push_str(&format!("inhibit_idle     = {}\n", cfg.inhibit_idle));
    out.push_str(&format!("inhibit_sleep    = {}\n", cfg.inhibit_sleep));
    out.push_str(&format!("notify_on_toggle = {}\n", cfg.notify_on_toggle));
    std::fs::write(path, out)
}
