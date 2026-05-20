// cosmic-caffeine: Wayland-native COSMIC panel applet for idle/sleep
// inhibition.
//
// The applet IS the long-running process. Click the panel button →
// popover with the on/off toggle, duration buttons (5/30/60 min,
// indefinite), and a Settings… link. The actual logind inhibit FD
// is held in App state behind an Arc<Mutex<Option<Inhibitor>>> —
// dropping it releases the lock back to logind.

use cosmic::app::{Core, Task};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{event, keyboard, window, Event, Length, Subscription};
use cosmic::prelude::*;
use cosmic::widget;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cosmic_caffeine::config;
use cosmic_caffeine::fl;
use cosmic_caffeine::inhibit::Inhibitor;
use cosmic_caffeine::localize;
use cosmic_caffeine::APP_ID;

const ICON_OFF: &str = "cosmic-caffeine-symbolic";
const ICON_ON: &str = "cosmic-caffeine-active-symbolic";

fn main() -> cosmic::iced::Result {
    localize::localize();
    cosmic::applet::run::<App>(())
}

#[derive(Clone, Debug)]
pub enum Message {
    TogglePopup,
    PopupClosed(window::Id),
    /// Acquire inhibit for `minutes` (0 = indefinite).
    Acquire(u32),
    /// Result of the async acquire — Ok updates state, Err clears it.
    AcquireResult(Result<u32, String>),
    /// Drop the inhibit FD now (user clicked "Turn off" or timer fired).
    Release,
    /// Auto-off timer for generation `gen` fired; release iff still
    /// active and the generation matches (i.e., user didn't change
    /// duration / toggle off in the meantime).
    TimerExpired(usize),
    OpenSettings,
    Noop,
}

pub struct App {
    core: Core,
    popup: Option<window::Id>,
    /// `Some(inh)` while the lock is held. The FD on the Inhibitor
    /// holds the logind block; dropping it releases.
    inhibitor: Arc<Mutex<Option<Inhibitor>>>,
    /// Wall-clock minutes the user requested for the current
    /// session. 0 = indefinite. Drives the popover label.
    active_minutes: u32,
    /// Bumped on every Acquire/Release so a still-pending
    /// TimerExpired from a stale generation no-ops.
    timer_generation: Arc<AtomicUsize>,
}

impl App {
    fn is_active(&self) -> bool {
        self.inhibitor.lock().unwrap().is_some()
    }

    fn close_popup_task(&mut self) -> Task<Message> {
        if let Some(p) = self.popup.take() {
            destroy_popup(p)
        } else {
            Task::none()
        }
    }
}

impl cosmic::Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _: ()) -> (Self, Task<Message>) {
        (
            App {
                core,
                popup: None,
                inhibitor: Arc::new(Mutex::new(None)),
                active_minutes: 0,
                timer_generation: Arc::new(AtomicUsize::new(0)),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TogglePopup => {
                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                }
                let new_id = window::Id::unique();
                self.popup = Some(new_id);
                let popup_settings = self.core.applet.get_popup_settings(
                    self.core.main_window_id().expect("applet has main window"),
                    new_id,
                    None,
                    None,
                    None,
                );
                get_popup(popup_settings)
            }
            Message::PopupClosed(id) => {
                if Some(id) == self.popup {
                    self.popup = None;
                }
                Task::none()
            }
            Message::Acquire(minutes) => {
                let inhibitor_slot = self.inhibitor.clone();
                let cfg = config::load();
                let why = if minutes == 0 {
                    "User enabled cosmic-caffeine indefinitely".to_string()
                } else {
                    format!("User enabled cosmic-caffeine for {minutes} minutes")
                };
                Task::perform(
                    async move {
                        match Inhibitor::acquire(cfg.inhibit_idle, cfg.inhibit_sleep, &why).await {
                            Ok(inh) => {
                                *inhibitor_slot.lock().unwrap() = Some(inh);
                                Ok(minutes)
                            }
                            Err(e) => Err(e),
                        }
                    },
                    |result| cosmic::Action::App(Message::AcquireResult(result)),
                )
            }
            Message::AcquireResult(Ok(minutes)) => {
                self.active_minutes = minutes;
                if config::load().notify_on_toggle {
                    let summary = if minutes == 0 {
                        fl!("notify-on-indefinite")
                    } else {
                        fl!("notify-on-minutes", minutes = minutes)
                    };
                    spawn_toggle_notification(summary);
                }
                let gen = self.timer_generation.fetch_add(1, Ordering::SeqCst) + 1;
                if minutes > 0 {
                    let timer_gen = self.timer_generation.clone();
                    return Task::perform(
                        async move {
                            tokio::time::sleep(Duration::from_secs(u64::from(minutes) * 60)).await;
                            // Only fire if the generation hasn't moved.
                            if timer_gen.load(Ordering::SeqCst) == gen {
                                Some(gen)
                            } else {
                                None
                            }
                        },
                        |result| match result {
                            Some(gen) => cosmic::Action::App(Message::TimerExpired(gen)),
                            None => cosmic::Action::App(Message::Noop),
                        },
                    );
                }
                Task::none()
            }
            Message::AcquireResult(Err(e)) => {
                eprintln!("cosmic-caffeine: acquire failed: {e}");
                self.active_minutes = 0;
                Task::none()
            }
            Message::Release => {
                let was_active = self.inhibitor.lock().unwrap().is_some();
                *self.inhibitor.lock().unwrap() = None;
                self.active_minutes = 0;
                self.timer_generation.fetch_add(1, Ordering::SeqCst);
                if was_active && config::load().notify_on_toggle {
                    spawn_toggle_notification(fl!("notify-off"));
                }
                Task::none()
            }
            Message::TimerExpired(gen) => {
                if self.timer_generation.load(Ordering::SeqCst) == gen {
                    let was_active = self.inhibitor.lock().unwrap().is_some();
                    *self.inhibitor.lock().unwrap() = None;
                    self.active_minutes = 0;
                    if was_active && config::load().notify_on_toggle {
                        spawn_toggle_notification(fl!("notify-off"));
                    }
                }
                Task::none()
            }
            Message::OpenSettings => {
                if let Ok(exe) = std::env::current_exe() {
                    let settings_bin = exe
                        .parent()
                        .map(|p| p.join("cosmic-caffeine-settings"))
                        .unwrap_or_else(|| std::path::PathBuf::from("cosmic-caffeine-settings"));
                    let _ = Command::new(settings_bin).spawn();
                }
                self.close_popup_task()
            }
            Message::Noop => Task::none(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let icon = if self.is_active() { ICON_ON } else { ICON_OFF };
        self.core
            .applet
            .icon_button(icon)
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: window::Id) -> Element<'_, Message> {
        let active = self.is_active();
        let header = widget::text::heading(if active {
            if self.active_minutes == 0 {
                fl!("popup-on-indefinite")
            } else {
                fl!("popup-on", minutes = self.active_minutes)
            }
        } else {
            fl!("popup-off")
        });

        let toggle: Element<Message> = if active {
            widget::button::destructive(fl!("turn-off"))
                .on_press(Message::Release)
                .width(Length::Fill)
                .into()
        } else {
            widget::button::suggested(fl!("turn-on"))
                .on_press(Message::Acquire(config::load().default_minutes))
                .width(Length::Fill)
                .into()
        };

        let mut sections: Vec<Element<Message>> =
            vec![header.into(), toggle];

        if !active {
            // Show duration shortcuts only when off.
            let buttons: Vec<Element<Message>> = vec![
                widget::button::standard(fl!("on-for", minutes = 5u32))
                    .on_press(Message::Acquire(5))
                    .into(),
                widget::button::standard(fl!("on-for", minutes = 30u32))
                    .on_press(Message::Acquire(30))
                    .into(),
                widget::button::standard(fl!("on-for", minutes = 60u32))
                    .on_press(Message::Acquire(60))
                    .into(),
                widget::button::standard(fl!("on-indefinitely"))
                    .on_press(Message::Acquire(0))
                    .into(),
            ];
            sections.push(widget::column::with_children(buttons).spacing(4).into());
        }

        let footer = widget::row::with_children(vec![
            widget::button::standard(fl!("settings"))
                .on_press(Message::OpenSettings)
                .into(),
            widget::space::horizontal().into(),
        ])
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center);

        sections.push(widget::space::vertical().height(Length::Fixed(4.0)).into());
        sections.push(footer.into());

        let content = widget::column::with_children(sections).spacing(8).padding(8);
        self.core.applet.popup_container(content).into()
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn subscription(&self) -> Subscription<Message> {
        event::listen_with(|evt, _status, _id| match evt {
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                if matches!(key, keyboard::Key::Named(keyboard::key::Named::Escape)) {
                    Some(Message::Noop)
                } else {
                    None
                }
            }
            _ => None,
        })
    }
}

/// Fire a "Caffeine on/off" desktop notification. Spawned on a blocking
/// thread because `notify_rust::Notification::show` makes a synchronous
/// D-Bus call.
fn spawn_toggle_notification(summary: String) {
    std::thread::spawn(move || {
        let _ = notify_rust::Notification::new()
            .summary(&summary)
            .icon(ICON_ON)
            .show();
    });
}
