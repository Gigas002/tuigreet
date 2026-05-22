use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use crate::Greeter;

use super::menu::MenuItem;

#[derive(Default)]
pub enum SessionSource {
    #[default]
    None,
    DefaultCommand(String, Option<Vec<String>>),
    Command(String),
    Session(usize),
}

impl SessionSource {
    pub fn label<'g, 'ss: 'g>(&'ss self, greeter: &'g Greeter) -> Option<&'g str> {
        match self {
            SessionSource::None => None,
            SessionSource::DefaultCommand(command, _) => Some(command),
            SessionSource::Command(command) => Some(command),
            SessionSource::Session(index) => greeter
                .sessions
                .options
                .get(*index)
                .map(|session| session.name.as_str()),
        }
    }

    pub fn command<'g, 'ss: 'g>(&'ss self, greeter: &'g Greeter) -> Option<&'g str> {
        match self {
            SessionSource::None => None,
            SessionSource::DefaultCommand(command, _) => Some(command.as_str()),
            SessionSource::Command(command) => Some(command.as_str()),
            SessionSource::Session(index) => greeter
                .sessions
                .options
                .get(*index)
                .map(|session| session.command.as_str()),
        }
    }

    pub fn env<'g, 'ss: 'g>(&'ss self) -> Option<Vec<String>> {
        match self {
            SessionSource::None => None,
            SessionSource::DefaultCommand(_, env) => env.clone(),
            SessionSource::Command(_) => None,
            SessionSource::Session(_) => None,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum SessionType {
    X11,
    Wayland,
    Tty,
    #[default]
    None,
}

impl SessionType {
    pub fn as_xdg_session_type(&self) -> &'static str {
        match self {
            SessionType::X11 => "x11",
            SessionType::Wayland => "wayland",
            SessionType::Tty => "tty",
            SessionType::None => "unspecified",
        }
    }
}

#[derive(Clone, Default)]
pub struct Session {
    pub slug: Option<String>,
    pub name: String,
    pub command: String,
    pub session_type: SessionType,
    pub path: Option<PathBuf>,
    pub xdg_desktop_names: Option<String>,
}

impl MenuItem for Session {
    fn format(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }
}

impl Session {
    pub fn from_path<P>(greeter: &Greeter, path: P) -> Option<&Session>
    where
        P: AsRef<Path>,
    {
        greeter
            .sessions
            .options
            .iter()
            .find(|session| session.path.as_deref() == Some(path.as_ref()))
    }

    pub fn get_selected(greeter: &Greeter) -> Option<&Session> {
        match greeter.session_source {
            SessionSource::Session(index) => greeter.sessions.options.get(index),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
