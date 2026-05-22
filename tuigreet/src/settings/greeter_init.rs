use libtuigreet::{
    event::Event,
    greeter::{GreetAlign, Greeter, SecretDisplay},
    info::{
        get_issue, get_last_command, get_last_session_path, get_last_user_command,
        get_last_user_name, get_last_user_session, get_last_user_username, get_min_max_uids,
        get_sessions, get_users,
    },
    model::{
        masked::MaskedString,
        menu::Menu,
        power_item::Power,
        sessions::{Session, SessionSource, SessionType},
    },
    power::PowerOption,
};
use tokio::sync::mpsc::Sender;

use crate::ui::strings;

use super::{SessionKind, Settings};

impl From<crate::config::GreetAlign> for GreetAlign {
    fn from(value: crate::config::GreetAlign) -> Self {
        match value {
            crate::config::GreetAlign::Left => GreetAlign::Left,
            crate::config::GreetAlign::Center => GreetAlign::Center,
            crate::config::GreetAlign::Right => GreetAlign::Right,
        }
    }
}

pub async fn init_greeter(events: Sender<Event>, settings: &Settings) -> Greeter {
    let mut greeter = Greeter::default();
    greeter.events = Some(events);

    greeter.powers = Menu {
        title: strings::get("title_power"),
        options: Default::default(),
        selected: 0,
    };

    apply_config(&mut greeter, settings);

    #[cfg(not(feature = "test-harness"))]
    {
        use std::process;

        match std::env::var("GREETD_SOCK") {
            Ok(socket) => greeter.socket = socket,
            Err(_) => {
                eprintln!("GREETD_SOCK must be defined");
                process::exit(1);
            }
        }

        greeter.connect().await;
    }

    greeter.logger = crate::logger::init(&greeter);

    let sessions = get_sessions(&greeter).unwrap_or_default();

    if let SessionSource::None = greeter.session_source {
        if !sessions.is_empty() {
            greeter.session_source = SessionSource::Session(0);
        }
    }

    greeter.sessions = Menu {
        title: strings::get("title_session"),
        options: sessions,
        selected: 0,
    };

    if greeter.remember {
        if let Some(username) = get_last_user_username() {
            greeter.username = MaskedString::from(username, get_last_user_name());

            if greeter.remember_user_session {
                if let Ok(command) = get_last_user_command(greeter.username.get()) {
                    greeter.session_source = SessionSource::Command(command);
                }

                if let Ok(ref session_path) = get_last_user_session(greeter.username.get()) {
                    if let Some(index) = greeter
                        .sessions
                        .options
                        .iter()
                        .position(|Session { path, .. }| path.as_deref() == Some(session_path))
                    {
                        greeter.sessions.selected = index;
                        greeter.session_source = SessionSource::Session(greeter.sessions.selected);
                    }
                }
            }
        }
    }

    if greeter.remember_session {
        if let Ok(command) = get_last_command() {
            greeter.session_source = SessionSource::Command(command.trim().to_string());
        }

        if let Ok(ref session_path) = get_last_session_path() {
            if let Some(index) = greeter
                .sessions
                .options
                .iter()
                .position(|Session { path, .. }| path.as_deref() == Some(session_path))
            {
                greeter.sessions.selected = index;
                greeter.session_source = SessionSource::Session(greeter.sessions.selected);
            }
        }
    }

    greeter
}

fn apply_config(greeter: &mut Greeter, settings: &Settings) {
    greeter.debug = settings.logging.debug;
    greeter.logfile = settings.logging.file.clone();
    greeter.time = settings.ui.show_time;
    greeter.time_format = settings.ui.time_format.clone();

    greeter.width = settings.ui.width;
    greeter.window_padding = settings.ui.window_padding;
    greeter.container_padding = settings.ui.container_padding.saturating_add(1);
    greeter.prompt_padding = settings.ui.prompt_padding;
    greeter.greet_align = settings.ui.greet_align.into();

    if settings.secrets.mask {
        greeter.secret_display = SecretDisplay::Character(settings.secrets.mask_char.clone());
    } else {
        greeter.secret_display = SecretDisplay::Hidden;
    }

    if settings.ui.issue {
        greeter.greeting = get_issue();
    } else {
        greeter.greeting = settings.ui.greeting.clone();
    }

    greeter.remember = settings.remember.username;
    greeter.remember_session = settings.remember.session;
    greeter.remember_user_session = settings.remember.user_session;

    greeter.user_menu = settings.user_menu.enabled;
    if settings.user_menu.enabled {
        let (min_uid, max_uid) = get_min_max_uids(
            Some(settings.user_menu.min_uid),
            Some(settings.user_menu.max_uid),
        );

        tracing::info!("min/max UIDs are {}/{}", min_uid, max_uid);

        greeter.users = Menu {
            title: strings::get("title_users"),
            options: get_users(min_uid, max_uid),
            selected: 0,
        };

        tracing::info!("found {} users", greeter.users.options.len());
    }

    greeter.session_paths = settings
        .session
        .session_paths
        .iter()
        .map(|(path, kind)| {
            let session_type = match kind {
                SessionKind::Wayland => SessionType::Wayland,
                SessionKind::X11 => SessionType::X11,
            };
            (path.clone(), session_type)
        })
        .collect();
    greeter.session_wrapper = settings.session.session_wrapper.clone();
    greeter.xsession_wrapper = settings.session.xsession_wrapper.clone();

    if let Some(command) = &settings.session.cmd {
        let envs = if settings.session.env.is_empty() {
            None
        } else {
            Some(settings.session.env.clone())
        };
        greeter.session_source = SessionSource::DefaultCommand(command.clone(), envs);
    }

    greeter.powers.options = vec![
        Power {
            action: PowerOption::Shutdown,
            label: strings::get("shutdown"),
            command: settings.power.shutdown.clone(),
        },
        Power {
            action: PowerOption::Reboot,
            label: strings::get("reboot"),
            command: settings.power.reboot.clone(),
        },
    ];
    greeter.power_setsid = settings.power.setsid;
    greeter.kb_command = settings.keybindings.command;
    greeter.kb_sessions = settings.keybindings.sessions;
    greeter.kb_power = settings.keybindings.power;
}
