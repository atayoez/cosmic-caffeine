// Standalone settings GUI for cosmic-caffeine. Launched as a child
// process by the applet's "Settings…" button.

use cosmic::app::{Core, Settings, Task};
use cosmic::iced::{Alignment, Length, Size};
use cosmic::prelude::*;
use cosmic::widget::{self, space};

use cosmic_caffeine::config::{self, CaffeineConfig};
use cosmic_caffeine::fl;
use cosmic_caffeine::localize;
use cosmic_caffeine::APP_ID;

#[derive(Clone, Debug, Default)]
#[allow(dead_code)] // Saving is matched in the view but only async saves
                    // construct it; caffeine's saves are synchronous.
enum SaveStatus {
    #[default]
    Idle,
    Saving,
    Saved,
    Error(String),
}

fn main() -> cosmic::iced::Result {
    localize::localize();
    let settings = Settings::default()
        .size(Size::new(640.0, 480.0))
        .exit_on_close(true);
    cosmic::app::run::<App>(settings, ())
}

#[derive(Clone, Debug)]
pub enum Message {
    DefaultMinutesText(String),
    InhibitIdle(bool),
    InhibitSleep(bool),
    NotifyOnToggle(bool),
    Save,
}

pub struct App {
    core: Core,
    default_minutes_text: String,
    inhibit_idle: bool,
    inhibit_sleep: bool,
    notify_on_toggle: bool,
    status: SaveStatus,
}

impl App {
    fn build_config(&self) -> CaffeineConfig {
        CaffeineConfig {
            default_minutes: self.default_minutes_text.parse().unwrap_or(0),
            inhibit_idle: self.inhibit_idle,
            inhibit_sleep: self.inhibit_sleep,
            notify_on_toggle: self.notify_on_toggle,
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
        let cfg = config::load();
        (
            App {
                core,
                default_minutes_text: cfg.default_minutes.to_string(),
                inhibit_idle: cfg.inhibit_idle,
                inhibit_sleep: cfg.inhibit_sleep,
                notify_on_toggle: cfg.notify_on_toggle,
                status: SaveStatus::Idle,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DefaultMinutesText(s) => {
                if s.chars().all(|c| c.is_ascii_digit()) && s.len() <= 4 {
                    self.default_minutes_text = s;
                }
            }
            Message::InhibitIdle(b) => self.inhibit_idle = b,
            Message::InhibitSleep(b) => self.inhibit_sleep = b,
            Message::NotifyOnToggle(b) => self.notify_on_toggle = b,
            Message::Save => {
                let cfg = self.build_config();
                self.status = match config::save(&cfg) {
                    Ok(()) => SaveStatus::Saved,
                    Err(e) => SaveStatus::Error(e.to_string()),
                };
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let behavior = widget::settings::section()
            .title(fl!("settings-section-behavior"))
            .add(widget::settings::item(
                fl!("settings-default-minutes"),
                widget::text_input("0", &self.default_minutes_text)
                    .on_input(Message::DefaultMinutesText)
                    .width(Length::Fixed(80.0)),
            ))
            .add(widget::settings::item(
                fl!("settings-inhibit-idle"),
                widget::toggler(self.inhibit_idle).on_toggle(Message::InhibitIdle),
            ))
            .add(widget::settings::item(
                fl!("settings-inhibit-sleep"),
                widget::toggler(self.inhibit_sleep).on_toggle(Message::InhibitSleep),
            ))
            .add(widget::settings::item(
                fl!("settings-notify-on-toggle"),
                widget::toggler(self.notify_on_toggle).on_toggle(Message::NotifyOnToggle),
            ));

        let body = widget::settings::view_column(vec![behavior.into()]);
        let scroll = widget::scrollable(body).height(Length::Fill);

        let status_widget: Element<Message> = match &self.status {
            SaveStatus::Idle => widget::Space::new().into(),
            SaveStatus::Saving => widget::text(fl!("settings-saving")).into(),
            SaveStatus::Saved => widget::text(fl!("settings-saved")).into(),
            SaveStatus::Error(e) => {
                widget::text(fl!("settings-error", error = e.clone())).into()
            }
        };

        let footer = widget::row::with_children(vec![
            status_widget,
            space::horizontal().into(),
            widget::button::suggested(fl!("settings-save"))
                .on_press(Message::Save)
                .into(),
        ])
        .spacing(8)
        .align_y(Alignment::Center);

        let content = widget::column::with_children(vec![scroll.into(), footer.into()])
            .spacing(16)
            .padding(16);

        widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
