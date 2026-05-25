use libratgreet::{
    event::Event,
    greeter::{GreetAlign, Greeter, SecretDisplay},
    info::{get_issue, get_sessions},
    model::{
        menu::Menu,
        power_item::Power,
        sessions::{SessionSource, SessionType},
    },
    power::PowerOption,
};
use tokio::sync::mpsc::Sender;

use crate::config::SecretDisplayMode;
use crate::settings::{SessionKind, Settings};
use crate::ui::strings;

impl From<crate::theme::GreetAlign> for GreetAlign {
    fn from(value: crate::theme::GreetAlign) -> Self {
        match value {
            crate::theme::GreetAlign::Left => GreetAlign::Left,
            crate::theme::GreetAlign::Center => GreetAlign::Center,
            crate::theme::GreetAlign::Right => GreetAlign::Right,
        }
    }
}

pub async fn init_greeter(events: Sender<Event>, settings: &Settings) -> Greeter {
    tracing::info!("init_greeter: start");

    let mut greeter = Greeter::default();
    greeter.events = Some(events);

    greeter.powers = Menu {
        title: strings::get("title_power"),
        options: Default::default(),
        selected: 0,
    };

    tracing::debug!("init_greeter: applying config");
    apply_config(&mut greeter, settings);
    tracing::debug!(
        debug = greeter.debug,
        logfile = %greeter.logfile,
        width = greeter.width,
        window_padding = greeter.window_padding,
        container_padding = greeter.container_padding,
        prompt_padding = greeter.prompt_padding,
        kb_command = greeter.kb_command,
        kb_sessions = greeter.kb_sessions,
        kb_power = greeter.kb_power,
        "init_greeter: config applied"
    );

    #[cfg(not(feature = "test-harness"))]
    {
        use std::process;

        match std::env::var("GREETD_SOCK") {
            Ok(socket) => {
                tracing::info!(socket = %socket, "init_greeter: GREETD_SOCK found");
                greeter.socket = socket;
            }
            Err(_) => {
                tracing::error!("init_greeter: GREETD_SOCK not set — exiting");
                eprintln!("GREETD_SOCK must be defined");
                process::exit(1);
            }
        }

        tracing::debug!("init_greeter: connecting to greetd");
        greeter.connect().await;
        tracing::info!("init_greeter: connected to greetd");
    }

    #[cfg(feature = "test-harness")]
    tracing::info!("init_greeter: test-harness mode — skipping greetd connection");

    tracing::debug!("init_greeter: initializing logger");
    greeter.logger = crate::logger::init(&greeter);
    tracing::info!(logger_active = greeter.logger.is_some(), "init_greeter: logger initialized");

    tracing::debug!("init_greeter: loading sessions");
    let sessions = get_sessions(&greeter).unwrap_or_default();
    tracing::info!(count = sessions.len(), "init_greeter: sessions loaded");

    if let SessionSource::None = greeter.session_source
        && !sessions.is_empty()
    {
        tracing::debug!("init_greeter: setting default session source to Session(0)");
        greeter.session_source = SessionSource::Session(0);
    }

    greeter.sessions = Menu {
        title: strings::get("title_session"),
        options: sessions,
        selected: 0,
    };

    tracing::info!("init_greeter: done");
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

    greeter.secret_display = match settings.secrets.display {
        SecretDisplayMode::Hidden => SecretDisplay::Hidden,
        SecretDisplayMode::Plain => SecretDisplay::Plain,
        SecretDisplayMode::Masked => SecretDisplay::Masked(settings.secrets.mask_char),
    };

    if settings.ui.issue {
        greeter.greeting = get_issue();
    } else {
        greeter.greeting = settings.ui.greeting.clone();
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
