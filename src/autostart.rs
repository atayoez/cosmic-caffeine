use crate::paths::{autostart_path, self_exec};

pub fn is_enabled() -> bool {
    autostart_path().exists()
}

pub fn enable() -> std::io::Result<()> {
    let path = autostart_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let exec = self_exec();
    let contents = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=cosmic-caffeine\n\
         GenericName=Idle/Sleep Inhibitor\n\
         Comment=Toggle idle and sleep inhibition (Wayland-native tray)\n\
         Exec={exec}\n\
         Icon=cosmic-caffeine-symbolic\n\
         Terminal=false\n\
         Categories=Utility;\n\
         StartupNotify=false\n\
         X-GNOME-Autostart-enabled=true\n\
         X-GNOME-Autostart-Delay=8\n"
    );
    std::fs::write(&path, contents)
}

pub fn disable() -> std::io::Result<()> {
    let path = autostart_path();
    if path.exists() {
        std::fs::remove_file(path)
    } else {
        Ok(())
    }
}
