use std::{
    error::Error,
    fmt::{self, Display},
    path::PathBuf,
    process,
    sync::Arc,
};

use tokio::{
    net::UnixStream,
    sync::{mpsc::Sender, RwLock, RwLockWriteGuard},
};
use tracing_appender::non_blocking::WorkerGuard;
use zeroize::Zeroize;

use crate::{
    event::Event,
    info::{
        get_issue, get_last_command, get_last_session_path, get_last_user_command,
        get_last_user_name, get_last_user_session, get_last_user_username, get_min_max_uids,
        get_sessions, get_users,
    },
    power::PowerOption,
    settings::{SessionKind, Settings},
    ui::{
        common::{masked::MaskedString, menu::Menu, style::Theme},
        power::Power,
        sessions::{Session, SessionSource, SessionType},
        users::User,
    },
};

pub use crate::config::GreetAlign;

#[derive(Debug, Copy, Clone)]
pub enum AuthStatus {
    Success,
    Failure,
    Cancel,
}

impl Display for AuthStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for AuthStatus {}

// A mode represents the large section of the software, usually screens to be
// displayed, or the state of the application.
#[derive(SmartDefault, Debug, Copy, Clone, PartialEq)]
pub enum Mode {
    #[default]
    Username,
    Password,
    Action,
    Users,
    Command,
    Sessions,
    Power,
    Processing,
}

// This enum models how secret values should be displayed on terminal.
#[derive(SmartDefault, Debug, Clone)]
pub enum SecretDisplay {
    #[default]
    // All characters hidden.
    Hidden,
    // All characters are replaced by a placeholder character.
    Character(String),
}

impl SecretDisplay {
    pub fn show(&self) -> bool {
        match self {
            SecretDisplay::Hidden => false,
            SecretDisplay::Character(_) => true,
        }
    }
}

#[derive(SmartDefault)]
pub struct Greeter {
    pub debug: bool,
    pub logfile: String,
    pub logger: Option<WorkerGuard>,

    #[default(80)]
    pub width: u16,
    pub window_padding: u16,
    #[default(2)]
    pub container_padding: u16,
    #[default(1)]
    pub prompt_padding: u16,
    pub greet_align: GreetAlign,

    pub socket: String,
    pub stream: Option<Arc<RwLock<UnixStream>>>,
    pub events: Option<Sender<Event>>,

    // Current mode of the application, will define what actions are permitted.
    pub mode: Mode,
    // Mode the application will return to when exiting the current mode.
    pub previous_mode: Mode,
    // Offset the cursor should be at from its base position for the current mode.
    pub cursor_offset: i16,

    // Buffer to be used as a temporary editing zone for the various modes.
    // Previous buffer is saved when a transient screen has to use the buffer, to
    // be able to restore it when leaving the transient screen.
    pub previous_buffer: Option<String>,
    pub buffer: String,

    // Define the selected session and how to resolve it.
    pub session_source: SessionSource,
    // List of session files found on disk.
    pub session_paths: Vec<(PathBuf, SessionType)>,
    // Menu for session selection.
    pub sessions: Menu<Session>,
    // Wrapper command to prepend to non-X11 sessions.
    pub session_wrapper: Option<String>,
    // Wrapper command to prepend to X11 sessions.
    pub xsession_wrapper: Option<String>,

    // Whether user menu is enabled.
    pub user_menu: bool,
    // Menu for user selection.
    pub users: Menu<User>,
    // Current username. Masked to display the full name if available.
    pub username: MaskedString,
    // Prompt that should be displayed to ask for entry.
    pub prompt: Option<String>,

    // Whether the current edition prompt should be hidden.
    pub asking_for_secret: bool,
    // How should secrets be displayed?
    pub secret_display: SecretDisplay,

    // Whether last logged-in user should be remembered.
    pub remember: bool,
    // Whether last launched session (regardless of user) should be remembered.
    pub remember_session: bool,
    // Whether last launched session for the current user should be remembered.
    pub remember_user_session: bool,

    // Style object for the terminal UI
    pub theme: Theme,
    // Display the current time
    pub time: bool,
    // Time format
    pub time_format: Option<String>,
    // Greeting message (MOTD) to use to welcome the user.
    pub greeting: Option<String>,
    // Transaction message to show to the user.
    pub message: Option<String>,

    // Menu for power options.
    pub powers: Menu<Power>,
    // Whether to prefix the power commands with `setsid`.
    pub power_setsid: bool,

    #[default(2)]
    pub kb_command: u8,
    #[default(3)]
    pub kb_sessions: u8,
    #[default(12)]
    pub kb_power: u8,

    // The software is waiting for a response from `greetd`.
    pub working: bool,
    // We are done working.
    pub done: bool,
    // Should we exit?
    pub exit: Option<AuthStatus>,
}

impl Drop for Greeter {
    fn drop(&mut self) {
        self.scrub(true, false);
    }
}

impl Greeter {
    pub async fn new(events: Sender<Event>, settings: Settings) -> Self {
        let mut greeter = Self::default();

        greeter.events = Some(events);

        greeter.powers = Menu {
            title: fl!("title_power"),
            options: Default::default(),
            selected: 0,
        };

        greeter.apply_settings(&settings);

        #[cfg(not(test))]
        {
            match std::env::var("GREETD_SOCK") {
                Ok(socket) => greeter.socket = socket,
                Err(_) => {
                    eprintln!("GREETD_SOCK must be defined");
                    process::exit(1);
                }
            }

            greeter.connect().await;
        }

        greeter.logger = crate::init_logger(&greeter);

        let sessions = get_sessions(&greeter).unwrap_or_default();

        if let SessionSource::None = greeter.session_source {
            if !sessions.is_empty() {
                greeter.session_source = SessionSource::Session(0);
            }
        }

        greeter.sessions = Menu {
            title: fl!("title_session"),
            options: sessions,
            selected: 0,
        };

        // If we should remember the last logged-in user.
        if greeter.remember {
            if let Some(username) = get_last_user_username() {
                greeter.username = MaskedString::from(username, get_last_user_name());

                // If, on top of that, we should remember their last session.
                if greeter.remember_user_session {
                    // See if we have the last free-form command from the user.
                    if let Ok(command) = get_last_user_command(greeter.username.get()) {
                        greeter.session_source = SessionSource::Command(command);
                    }

                    // If a session was saved, use it and its name.
                    if let Ok(ref session_path) = get_last_user_session(greeter.username.get()) {
                        // Set the selected menu option and the session source.
                        if let Some(index) = greeter
                            .sessions
                            .options
                            .iter()
                            .position(|Session { path, .. }| path.as_deref() == Some(session_path))
                        {
                            greeter.sessions.selected = index;
                            greeter.session_source =
                                SessionSource::Session(greeter.sessions.selected);
                        }
                    }
                }
            }
        }

        // Same thing, but not user specific.
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

    // Scrub memory of all data, unless `soft` is true, in which case, we will
    // keep the username (can happen if a wrong password was entered, we want to
    // give the user another chance, as PAM would).
    fn scrub(&mut self, scrub_message: bool, soft: bool) {
        self.buffer.zeroize();
        self.prompt.zeroize();

        if !soft {
            self.username.zeroize();
        }

        if scrub_message {
            self.message.zeroize();
        }
    }

    // Reset the software to its initial state.
    pub async fn reset(&mut self, soft: bool) {
        if soft {
            self.mode = Mode::Password;
            self.previous_mode = Mode::Password;
        } else {
            self.mode = Mode::Username;
            self.previous_mode = Mode::Username;
        }

        self.working = false;
        self.done = false;

        self.scrub(false, soft);
        self.connect().await;
    }

    // Connect to `greetd` and return a stream we can safely write to.
    pub async fn connect(&mut self) {
        match UnixStream::connect(&self.socket).await {
            Ok(stream) => self.stream = Some(Arc::new(RwLock::new(stream))),

            Err(err) => {
                eprintln!("{err}");
                process::exit(1);
            }
        }
    }

    pub async fn stream(&self) -> RwLockWriteGuard<'_, UnixStream> {
        self.stream.as_ref().unwrap().write().await
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn window_padding(&self) -> u16 {
        self.window_padding
    }

    pub fn container_padding(&self) -> u16 {
        self.container_padding
    }

    pub fn prompt_padding(&self) -> u16 {
        self.prompt_padding
    }

    pub fn greet_align(&self) -> GreetAlign {
        self.greet_align
    }

    pub fn apply_settings(&mut self, settings: &Settings) {
        self.debug = settings.logging.debug;
        self.logfile = settings.logging.file.clone();
        self.theme = settings.theme.clone();
        self.time = settings.ui.show_time;
        self.time_format = settings.ui.time_format.clone();

        self.width = settings.ui.width;
        self.window_padding = settings.ui.window_padding;
        self.container_padding = settings.ui.container_padding.saturating_add(1);
        self.prompt_padding = settings.ui.prompt_padding;
        self.greet_align = settings.ui.greet_align;

        if settings.secrets.mask {
            self.secret_display = SecretDisplay::Character(settings.secrets.mask_char.clone());
        } else {
            self.secret_display = SecretDisplay::Hidden;
        }

        if settings.ui.issue {
            self.greeting = get_issue();
        } else {
            self.greeting = settings.ui.greeting.clone();
        }

        self.remember = settings.remember.username;
        self.remember_session = settings.remember.session;
        self.remember_user_session = settings.remember.user_session;

        self.user_menu = settings.user_menu.enabled;
        if settings.user_menu.enabled {
            let (min_uid, max_uid) = get_min_max_uids(
                Some(settings.user_menu.min_uid),
                Some(settings.user_menu.max_uid),
            );

            tracing::info!("min/max UIDs are {}/{}", min_uid, max_uid);

            self.users = Menu {
                title: fl!("title_users"),
                options: get_users(min_uid, max_uid),
                selected: 0,
            };

            tracing::info!("found {} users", self.users.options.len());
        }

        self.session_paths = settings
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
        self.session_wrapper = settings.session.session_wrapper.clone();
        self.xsession_wrapper = settings.session.xsession_wrapper.clone();

        if let Some(command) = &settings.session.cmd {
            let envs = if settings.session.env.is_empty() {
                None
            } else {
                Some(settings.session.env.clone())
            };
            self.session_source = SessionSource::DefaultCommand(command.clone(), envs);
        }

        self.powers.options = vec![
            Power {
                action: PowerOption::Shutdown,
                label: fl!("shutdown"),
                command: settings.power.shutdown.clone(),
            },
            Power {
                action: PowerOption::Reboot,
                label: fl!("reboot"),
                command: settings.power.reboot.clone(),
            },
        ];
        self.power_setsid = settings.power.setsid;
        self.kb_command = settings.keybindings.command;
        self.kb_sessions = settings.keybindings.sessions;
        self.kb_power = settings.keybindings.power;
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        self.prompt = if prompt.ends_with(' ') {
            Some(prompt.into())
        } else {
            Some(format!("{prompt} "))
        };
    }

    pub fn remove_prompt(&mut self) {
        self.prompt = None;
    }

    // Computes the size of the prompt to help determine where input should start.
    pub fn prompt_width(&self) -> usize {
        match &self.prompt {
            None => 0,
            Some(prompt) => prompt.chars().count(),
        }
    }
}

#[cfg(test)]
mod tests;
