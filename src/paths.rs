use std::path::PathBuf;

pub const APP_ID: &str = "io.github.atayozcan.CosmicCaffeine";

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .expect("no XDG_CONFIG_HOME")
        .join("cosmic-caffeine/config.toml")
}

pub fn autostart_path() -> PathBuf {
    dirs::config_dir()
        .expect("no XDG_CONFIG_HOME")
        .join("autostart/cosmic-caffeine.desktop")
}

pub fn self_exec() -> String {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "cosmic-caffeine".to_string())
}

pub fn settings_exec() -> Option<PathBuf> {
    if let Ok(self_path) = std::env::current_exe() {
        if let Some(parent) = self_path.parent() {
            let candidate = parent.join("cosmic-caffeine-settings");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    which("cosmic-caffeine-settings")
}

fn which(bin: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|p| p.join(bin))
            .find(|p| p.exists())
    })
}
