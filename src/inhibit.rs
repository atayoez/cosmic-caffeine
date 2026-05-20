//! Two-pronged idle/sleep inhibition.
//!
//! 1. `org.freedesktop.login1.Manager.Inhibit` on the SYSTEM bus
//!    returns a file descriptor. Closing the FD releases the lock.
//!    This is what stops automatic suspend.
//!
//! 2. `org.freedesktop.ScreenSaver.Inhibit` on the SESSION bus
//!    returns a cookie. The compositor (cosmic-comp, mutter, kwin,
//!    sway, …) typically manages screen blanking itself and honors
//!    *this* protocol, not logind's "idle" inhibit class. Without it,
//!    the screen still blanks even when the logind lock is held.
//!    The cookie is implicit on the connection — drop the
//!    Connection and the compositor releases it.
//!
//! `what` accepts a colon-separated set of: shutdown, sleep, idle,
//! handle-power-key, handle-suspend-key, handle-hibernate-key,
//! handle-lid-switch.
//!
//! Mode "block" means logind will refuse to perform the action while the
//! inhibitor is held; "delay" gives a grace period before proceeding.

use zbus::zvariant::OwnedFd;

pub struct Inhibitor {
    /// Holding this FD keeps the logind inhibit lock active. We never
    /// read/write to it; closing it (drop) releases the inhibition.
    _fd: OwnedFd,
    /// Holding this session-bus connection keeps an
    /// `org.freedesktop.ScreenSaver` inhibit cookie alive. `None` if
    /// idle-inhibit isn't requested or the session bus doesn't expose
    /// the ScreenSaver service (e.g., headless). Drop releases.
    _screensaver: Option<zbus::Connection>,
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

        let screensaver = if idle {
            acquire_screensaver(why).await
        } else {
            None
        };

        Ok(Self {
            _fd: fd,
            _screensaver: screensaver,
            what,
        })
    }
}

/// Best-effort `org.freedesktop.ScreenSaver.Inhibit`. Returns the session
/// bus connection we used; the compositor releases the cookie when the
/// connection drops. Returns `None` on any failure — a missing
/// ScreenSaver service shouldn't fail the whole acquire (logind's lock
/// is still useful on its own for the suspend case).
async fn acquire_screensaver(why: &str) -> Option<zbus::Connection> {
    let conn = zbus::Connection::session().await.ok()?;
    let proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.ScreenSaver",
        "/ScreenSaver",
        "org.freedesktop.ScreenSaver",
    )
    .await
    .ok()?;
    // Returns a `u` cookie; we don't need to keep it — the cookie is
    // tied to this connection, so dropping the connection releases.
    let _cookie: u32 = proxy.call("Inhibit", &("cosmic-caffeine", why)).await.ok()?;
    Some(conn)
}
