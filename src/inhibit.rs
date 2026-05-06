//! Wrapper around `org.freedesktop.login1.Manager.Inhibit`.
//!
//! systemd-logind exposes Inhibit() over the system bus. It returns a file
//! descriptor that, while held open, keeps the inhibition active. Drop the
//! FD and logind cancels it. We hold the FD on the [`Inhibitor`] handle.
//!
//! `what` accepts a colon-separated set of: shutdown, sleep, idle,
//! handle-power-key, handle-suspend-key, handle-hibernate-key,
//! handle-lid-switch.
//!
//! Mode "block" means logind will refuse to perform the action while the
//! inhibitor is held; "delay" gives a grace period before proceeding.

use zbus::zvariant::OwnedFd;

pub struct Inhibitor {
    /// Holding this FD keeps the inhibit lock active. We never read/write to
    /// it; closing it (drop) releases the inhibition.
    _fd: OwnedFd,
    pub what: String,
}

impl Inhibitor {
    pub async fn acquire(idle: bool, sleep: bool, why: &str) -> Result<Self, String> {
        let mut parts: Vec<&str> = Vec::new();
        if idle {
            parts.push("idle");
        }
        if sleep {
            parts.push("sleep");
        }
        if parts.is_empty() {
            // Nothing to inhibit means nothing to acquire — fall back to idle
            // so the toggle still has a visible effect.
            parts.push("idle");
        }
        let what = parts.join(":");

        let conn = zbus::Connection::system()
            .await
            .map_err(|e| format!("system bus: {e}"))?;
        let proxy = zbus::Proxy::new(
            &conn,
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
        )
        .await
        .map_err(|e| format!("login1 proxy: {e}"))?;

        let fd: OwnedFd = proxy
            .call("Inhibit", &(what.as_str(), "cosmic-caffeine", why, "block"))
            .await
            .map_err(|e| format!("Inhibit: {e}"))?;

        Ok(Self { _fd: fd, what })
    }
}
