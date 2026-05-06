// libcosmic settings GUI for cosmic-caffeine. Edits are kept in memory until
// Save; the Autostart toggle applies immediately.

use cosmic::app::{Core, Settings, Task};
use cosmic::iced::{Alignment, Length, Size};
use cosmic::prelude::*;
use cosmic::widget::{self, space};
use cosmic::Action;

use cosmic_caffeine::autostart;
use cosmic_caffeine::config::{self, Config};
use cosmic_caffeine::paths::{config_path, APP_ID};

fn main() -> cosmic::iced::Result {
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
    Autostart(bool),
    Save,
    SaveResult(Result<(), String>),
}

#[derive(Clone, Debug, Default)]
enum SaveStatus {
    #[default]
    Idle,
    Saving,
    Saved,
    Error(String),
}

pub struct App {
    core: Core,
    default_minutes_text: String,
    inhibit_idle: bool,
    inhibit_sleep: bool,
    notify_on_toggle: bool,
    autostart_enabled: bool,
    status: SaveStatus,
}

impl App {
    fn build_config(&self) -> Config {
        Config {
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
        let cfg = config::read(&config_path()).unwrap_or_default();
        let mut app = App {
            core,
            default_minutes_text: cfg.default_minutes.to_string(),
            inhibit_idle: cfg.inhibit_idle,
            inhibit_sleep: cfg.inhibit_sleep,
            notify_on_toggle: cfg.notify_on_toggle,
            autostart_enabled: autostart::is_enabled(),
            status: SaveStatus::Idle,
        };
        let title = app.set_window_title("cosmic-caffeine Settings".into());
        (app, title)
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
            Message::Autostart(on) => {
                let res = if on {
                    autostart::enable()
                } else {
                    autostart::disable()
                };
                if let Err(e) = res {
                    self.status = SaveStatus::Error(format!("autostart: {e}"));
                }
                self.autostart_enabled = autostart::is_enabled();
            }
            Message::Save => {
                self.status = SaveStatus::Saving;
                let cfg = self.build_config();
                let path = config_path();
                return Task::perform(
                    async move { config::write(&path, &cfg).map_err(|e| e.to_string()) },
                    |r| Action::App(Message::SaveResult(r)),
                );
            }
            Message::SaveResult(Ok(())) => self.status = SaveStatus::Saved,
            Message::SaveResult(Err(e)) => self.status = SaveStatus::Error(e),
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let behavior = widget::settings::section()
            .title("Behavior")
            .add(widget::settings::item(
                "Default duration on click (minutes, 0 = indefinite)",
                widget::text_input("0", &self.default_minutes_text)
                    .on_input(Message::DefaultMinutesText)
                    .width(Length::Fixed(80.0)),
            ))
            .add(widget::settings::item(
                "Inhibit idle (block screen blanking / lock-on-idle)",
                widget::toggler(self.inhibit_idle).on_toggle(Message::InhibitIdle),
            ))
            .add(widget::settings::item(
                "Inhibit sleep (block automatic suspend)",
                widget::toggler(self.inhibit_sleep).on_toggle(Message::InhibitSleep),
            ))
            .add(widget::settings::item(
                "Show notification on toggle",
                widget::toggler(self.notify_on_toggle).on_toggle(Message::NotifyOnToggle),
            ));

        let startup = widget::settings::section()
            .title("Startup")
            .add(widget::settings::item(
                "Start cosmic-caffeine on login",
                widget::toggler(self.autostart_enabled).on_toggle(Message::Autostart),
            ));

        let body = widget::settings::view_column(vec![behavior.into(), startup.into()]);
        let scroll = widget::scrollable(body).height(Length::Fill);

        let status_widget: Element<Message> = match &self.status {
            SaveStatus::Idle => widget::Space::new().into(),
            SaveStatus::Saving => widget::text("Saving…").into(),
            SaveStatus::Saved => widget::text("Saved.").into(),
            SaveStatus::Error(e) => widget::text(format!("Error: {e}")).into(),
        };

        let footer = widget::row::with_children(vec![
            status_widget,
            space::horizontal().into(),
            widget::button::suggested("Save")
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
