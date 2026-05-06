// cosmic-caffeine: Wayland-native idle/sleep inhibitor.
//
// - Click the tray icon to toggle inhibition (logind Inhibit() over D-Bus).
// - Tray menu picks duration (5/30/60 min or indefinite) and exposes
//   Settings… / Quit.
// - When the timer expires, inhibition releases and the tray icon flips
//   back to the inactive cup.
//
// The actual inhibit lock is a logind FD held on the InhibitState; dropping
// it returns the lock to logind.

use ksni::{menu::*, Tray, TrayMethods};
use notify_rust::Notification;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use tokio::time::{sleep, Duration};

use cosmic_caffeine::config::{self, Config};
use cosmic_caffeine::inhibit::Inhibitor;
use cosmic_caffeine::paths::{config_path, settings_exec};

#[derive(Clone)]
struct CaffeineTray {
    state: Arc<Mutex<TrayState>>,
}

#[derive(Default)]
struct TrayState {
    cfg: Config,
    inhibitor: Option<Inhibitor>,
    /// Send () to abort an in-flight auto-off timer (when the user toggles
    /// off manually before it fires).
    timer_abort: Option<oneshot::Sender<()>>,
    /// Wall-clock minute count requested for the current session, for the
    /// tooltip. 0 = indefinite.
    active_minutes: u32,
}

impl Tray for CaffeineTray {
    fn id(&self) -> String {
        "cosmic-caffeine".into()
    }
    fn title(&self) -> String {
        "Caffeine".into()
    }
    fn icon_name(&self) -> String {
        let active = self
            .state
            .lock()
            .map(|s| s.inhibitor.is_some())
            .unwrap_or(false);
        if active {
            "cosmic-caffeine-active-symbolic".into()
        } else {
            "cosmic-caffeine-symbolic".into()
        }
    }
    fn icon_theme_path(&self) -> String {
        String::new()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let (active, minutes) = self
            .state
            .lock()
            .map(|s| (s.inhibitor.is_some(), s.active_minutes))
            .unwrap_or((false, 0));
        let title = if active {
            if minutes == 0 {
                "Caffeine: on (indefinite)".into()
            } else {
                format!("Caffeine: on ({minutes} min)")
            }
        } else {
            "Caffeine: off".into()
        };
        ksni::ToolTip {
            title,
            description: "Click to toggle idle/sleep inhibition".into(),
            icon_name: if active {
                "cosmic-caffeine-active-symbolic".into()
            } else {
                "cosmic-caffeine-symbolic".into()
            },
            icon_pixmap: vec![],
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        // Default click — toggle with the configured default minutes.
        let minutes = self
            .state
            .lock()
            .map(|s| s.cfg.default_minutes)
            .unwrap_or(0);
        spawn_toggle(self.state.clone(), minutes);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let active = self
            .state
            .lock()
            .map(|s| s.inhibitor.is_some())
            .unwrap_or(false);

        let toggle_label = if active { "Turn off" } else { "Turn on" };
        let mut items: Vec<MenuItem<Self>> = vec![
            StandardItem {
                label: toggle_label.into(),
                icon_name: if active {
                    "media-playback-stop-symbolic".into()
                } else {
                    "media-playback-start-symbolic".into()
                },
                activate: Box::new(|t: &mut CaffeineTray| {
                    let minutes = t
                        .state
                        .lock()
                        .map(|s| s.cfg.default_minutes)
                        .unwrap_or(0);
                    spawn_toggle(t.state.clone(), minutes);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
        ];

        if !active {
            for &mins in &[5u32, 30, 60] {
                items.push(
                    StandardItem {
                        label: format!("On for {mins} min"),
                        activate: Box::new(move |t: &mut CaffeineTray| {
                            spawn_toggle(t.state.clone(), mins);
                        }),
                        ..Default::default()
                    }
                    .into(),
                );
            }
            items.push(
                StandardItem {
                    label: "On indefinitely".into(),
                    activate: Box::new(|t: &mut CaffeineTray| {
                        spawn_toggle(t.state.clone(), 0);
                    }),
                    ..Default::default()
                }
                .into(),
            );
            items.push(MenuItem::Separator);
        }

        items.push(
            StandardItem {
                label: "Settings...".into(),
                icon_name: "preferences-system-symbolic".into(),
                activate: Box::new(|_| {
                    if let Some(exe) = settings_exec() {
                        let _ = Command::new(exe).spawn();
                    } else {
                        let _ = Command::new("xdg-open").arg(config_path()).spawn();
                    }
                }),
                ..Default::default()
            }
            .into(),
        );
        items.push(MenuItem::Separator);
        items.push(
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit-symbolic".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        );
        items
    }
}

/// Toggle. If currently active, releases the lock; otherwise acquires it
/// and (if `minutes > 0`) schedules an auto-off timer.
fn spawn_toggle(state: Arc<Mutex<TrayState>>, minutes: u32) {
    tokio::spawn(async move {
        let was_active = state
            .lock()
            .map(|s| s.inhibitor.is_some())
            .unwrap_or(false);

        if was_active {
            release(&state);
        } else if let Err(e) = acquire(&state, minutes).await {
            eprintln!("cosmic-caffeine: acquire failed: {e}");
            notify_error(&e);
        }
    });
}

async fn acquire(state: &Arc<Mutex<TrayState>>, minutes: u32) -> Result<(), String> {
    let (idle, sleep_inh, notify, why) = {
        let s = state.lock().map_err(|_| "state lock poisoned")?;
        (
            s.cfg.inhibit_idle,
            s.cfg.inhibit_sleep,
            s.cfg.notify_on_toggle,
            if minutes == 0 {
                "User enabled cosmic-caffeine indefinitely".to_string()
            } else {
                format!("User enabled cosmic-caffeine for {minutes} minutes")
            },
        )
    };

    let inhibitor = Inhibitor::acquire(idle, sleep_inh, &why).await?;
    let what = inhibitor.what.clone();

    let (tx, rx) = oneshot::channel::<()>();
    {
        let mut s = state.lock().map_err(|_| "state lock poisoned")?;
        s.inhibitor = Some(inhibitor);
        s.active_minutes = minutes;
        s.timer_abort = Some(tx);
    }

    if notify {
        let body = if minutes == 0 {
            format!("Inhibiting {what} indefinitely")
        } else {
            format!("Inhibiting {what} for {minutes} min")
        };
        let _ = Notification::new()
            .summary("Caffeine on")
            .body(&body)
            .icon("cosmic-caffeine-active-symbolic")
            .appname("cosmic-caffeine")
            .show();
    }

    if minutes > 0 {
        let state = state.clone();
        tokio::spawn(async move {
            let timeout = Duration::from_secs(u64::from(minutes) * 60);
            tokio::select! {
                _ = sleep(timeout) => {
                    release(&state);
                    let notify = state.lock().map(|s| s.cfg.notify_on_toggle).unwrap_or(false);
                    if notify {
                        let _ = Notification::new()
                            .summary("Caffeine off")
                            .body("Timer expired; idle/sleep restored.")
                            .icon("cosmic-caffeine-symbolic")
                            .appname("cosmic-caffeine")
                            .show();
                    }
                }
                _ = rx => {
                    // User toggled off manually before the timer.
                }
            }
        });
    }

    Ok(())
}

fn release(state: &Arc<Mutex<TrayState>>) {
    let (notify, was_active) = {
        let mut s = match state.lock() {
            Ok(s) => s,
            Err(_) => return,
        };
        let was_active = s.inhibitor.is_some();
        s.inhibitor = None;
        s.active_minutes = 0;
        if let Some(tx) = s.timer_abort.take() {
            let _ = tx.send(());
        }
        (s.cfg.notify_on_toggle, was_active)
    };
    if was_active && notify {
        let _ = Notification::new()
            .summary("Caffeine off")
            .body("Idle/sleep restored.")
            .icon("cosmic-caffeine-symbolic")
            .appname("cosmic-caffeine")
            .show();
    }
}

fn notify_error(msg: &str) {
    let _ = Notification::new()
        .summary("cosmic-caffeine: error")
        .body(msg)
        .appname("cosmic-caffeine")
        .show();
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    if let Some(cmd) = args.next() {
        match cmd.as_str() {
            "--help" | "-h" => {
                println!("cosmic-caffeine: Wayland-native idle/sleep inhibitor.");
                println!();
                println!("Usage:");
                println!("  cosmic-caffeine            run the tray daemon");
                println!("  cosmic-caffeine --help     this help");
                println!();
                println!("Run `cosmic-caffeine-settings` for the GUI settings editor.");
                return Ok(());
            }
            other => {
                eprintln!("cosmic-caffeine: unknown argument {other:?}");
                std::process::exit(2);
            }
        }
    }

    let cfg_path = config_path();
    let cfg = config::read(&cfg_path).unwrap_or_default();
    if !cfg_path.exists() {
        let _ = config::write(&cfg_path, &cfg);
    }

    let state = Arc::new(Mutex::new(TrayState {
        cfg,
        ..Default::default()
    }));

    let tray = CaffeineTray {
        state: state.clone(),
    };
    let _handle = tray.spawn().await?;

    std::future::pending::<()>().await;
    Ok(())
}
