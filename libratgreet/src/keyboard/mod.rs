use std::{error::Error, sync::Arc};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use greetd_ipc::Request;
use tokio::sync::RwLock;

use crate::{
    Greeter, Mode,
    ipc::Ipc,
    model::{
        masked::MaskedString,
        sessions::{Session, SessionSource},
    },
    power::power,
};

pub async fn handle(
    greeter: Arc<RwLock<Greeter>>,
    input: KeyEvent,
    ipc: Ipc,
) -> Result<(), Box<dyn Error>> {
    let mut greeter = greeter.write().await;

    tracing::debug!(mode = ?greeter.mode, key = ?input.code, mods = ?input.modifiers, working = greeter.working, "keyboard::handle");

    if greeter.working {
        tracing::debug!("keyboard::handle: greeter.working=true, ignoring key");
        return Ok(());
    }

    match input {
        KeyEvent {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => {
            tracing::debug!(mode = ?greeter.mode, "keyboard: Ctrl+U — erase buffer");
            match greeter.mode {
                Mode::Username => greeter.username = MaskedString::default(),
                Mode::Password => greeter.buffer = String::new(),
                Mode::Command => greeter.buffer = String::new(),
                _ => {}
            }
        }

        #[cfg(debug_assertions)]
        KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => {
            use crate::{AuthStatus, Event};

            tracing::debug!("keyboard: Ctrl+X (debug) — sending Exit(Cancel)");
            if let Some(ref sender) = greeter.events {
                let _ = sender.send(Event::Exit(AuthStatus::Cancel)).await;
            }
        }

        KeyEvent {
            code: KeyCode::Esc, ..
        } => {
            tracing::debug!(mode = ?greeter.mode, "keyboard: Esc");
            match greeter.mode {
                Mode::Command => {
                    tracing::debug!("keyboard: Esc in Command — returning to previous mode");
                    greeter.mode = greeter.previous_mode;
                    greeter.buffer = greeter.previous_buffer.take().unwrap_or_default();
                    greeter.cursor_offset = 0;
                }

                Mode::Sessions | Mode::Power => {
                    tracing::debug!(mode = ?greeter.mode, "keyboard: Esc in Sessions/Power — returning to previous mode");
                    greeter.mode = greeter.previous_mode;
                }

                Mode::Username => {
                    tracing::debug!("keyboard: Esc in Username — clearing username buffer");
                    greeter.username = MaskedString::default();
                    greeter.cursor_offset = 0;
                }

                _ => {
                    tracing::debug!(mode = ?greeter.mode, "keyboard: Esc in other mode — IPC cancel + reset");
                    Ipc::cancel(&mut greeter).await;
                    greeter.reset(false).await;
                }
            }
        }

        KeyEvent {
            code: KeyCode::Left,
            ..
        } => {
            tracing::debug!("keyboard: Left");
            greeter.cursor_offset -= 1;
        }

        KeyEvent {
            code: KeyCode::Right,
            ..
        } => {
            tracing::debug!("keyboard: Right");
            greeter.cursor_offset += 1;
        }

        KeyEvent {
            code: KeyCode::F(i),
            ..
        } if i == greeter.kb_command => {
            tracing::debug!(f = i, "keyboard: F(command)");
            greeter.previous_mode = match greeter.mode {
                Mode::Command | Mode::Sessions | Mode::Power => greeter.previous_mode,
                _ => greeter.mode,
            };
            greeter.previous_buffer = Some(greeter.buffer.clone());
            greeter.buffer = greeter
                .session_source
                .command(&greeter)
                .map(str::to_string)
                .unwrap_or_default();
            greeter.cursor_offset = 0;
            greeter.mode = Mode::Command;
        }

        KeyEvent {
            code: KeyCode::F(i),
            ..
        } if i == greeter.kb_sessions => {
            tracing::debug!(f = i, "keyboard: F(sessions)");
            greeter.previous_mode = match greeter.mode {
                Mode::Command | Mode::Sessions | Mode::Power => greeter.previous_mode,
                _ => greeter.mode,
            };
            greeter.mode = Mode::Sessions;
        }

        KeyEvent {
            code: KeyCode::F(i),
            ..
        } if i == greeter.kb_power => {
            tracing::debug!(f = i, "keyboard: F(power)");
            greeter.previous_mode = match greeter.mode {
                Mode::Command | Mode::Sessions | Mode::Power => greeter.previous_mode,
                _ => greeter.mode,
            };
            greeter.mode = Mode::Power;
        }

        KeyEvent {
            code: KeyCode::Up, ..
        } => {
            tracing::debug!("keyboard: Up");
            if let Mode::Sessions = greeter.mode
                && greeter.sessions.selected > 0
            {
                greeter.sessions.selected -= 1;
            }

            if let Mode::Power = greeter.mode
                && greeter.powers.selected > 0
            {
                greeter.powers.selected -= 1;
            }
        }

        KeyEvent {
            code: KeyCode::Down,
            ..
        } => {
            tracing::debug!("keyboard: Down");
            if let Mode::Sessions = greeter.mode
                && greeter.sessions.selected < greeter.sessions.options.len() - 1
            {
                greeter.sessions.selected += 1;
            }

            if let Mode::Power = greeter.mode
                && greeter.powers.selected < greeter.powers.options.len() - 1
            {
                greeter.powers.selected += 1;
            }
        }

        KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => {
            tracing::debug!("keyboard: Ctrl+A — jump to start");
            let value = {
                match greeter.mode {
                    Mode::Username => &greeter.username.value,
                    _ => &greeter.buffer,
                }
            };
            greeter.cursor_offset = -(value.chars().count() as i16);
        }

        KeyEvent {
            code: KeyCode::Char('e'),
            modifiers: KeyModifiers::CONTROL,
            ..
        } => {
            tracing::debug!("keyboard: Ctrl+E — jump to end");
            greeter.cursor_offset = 0;
        }

        KeyEvent {
            code: KeyCode::Tab, ..
        } => {
            tracing::debug!(mode = ?greeter.mode, "keyboard: Tab");
            match greeter.mode {
                Mode::Username if !greeter.username.value.is_empty() => {
                    tracing::debug!("keyboard: Tab in Username (non-empty) — validate_username");
                    validate_username(&mut greeter, &ipc).await
                }
                _ => {
                    tracing::debug!("keyboard: Tab — no action");
                }
            }
        }

        KeyEvent {
            code: KeyCode::Enter,
            ..
        } => {
            tracing::debug!(mode = ?greeter.mode, "keyboard: Enter");
            match greeter.mode {
                Mode::Username if !greeter.username.value.is_empty() => {
                    tracing::debug!("keyboard: Enter in Username (non-empty) — validate_username");
                    validate_username(&mut greeter, &ipc).await
                }

                Mode::Username => {
                    tracing::debug!("keyboard: Enter in Username (empty) — no action");
                }

                Mode::Password => {
                    tracing::debug!("keyboard: Enter in Password — posting auth response");
                    greeter.working = true;
                    greeter.message = None;

                    ipc.send(Request::PostAuthMessageResponse {
                        response: Some(greeter.buffer.clone()),
                    })
                    .await;

                    greeter.buffer = String::new();
                }

                Mode::Command => {
                    tracing::debug!("keyboard: Enter in Command — setting session source");
                    greeter.sessions.selected = 0;
                    greeter.session_source = SessionSource::Command(greeter.buffer.clone());

                    greeter.buffer = greeter.previous_buffer.take().unwrap_or_default();
                    greeter.mode = greeter.previous_mode;
                }

                Mode::Sessions => {
                    tracing::debug!(selected = greeter.sessions.selected, "keyboard: Enter in Sessions");
                    let session = greeter
                        .sessions
                        .options
                        .get(greeter.sessions.selected)
                        .cloned();

                    if let Some(Session { .. }) = session {
                        greeter.session_source = SessionSource::Session(greeter.sessions.selected);
                    }

                    greeter.mode = greeter.previous_mode;
                }

                Mode::Power => {
                    tracing::debug!(selected = greeter.powers.selected, "keyboard: Enter in Power");
                    let power_command = greeter.powers.options.get(greeter.powers.selected).cloned();

                    if let Some(command) = power_command {
                        power(&mut greeter, command.action).await;
                    }

                    greeter.mode = greeter.previous_mode;
                }

                _ => {
                    tracing::debug!(mode = ?greeter.mode, "keyboard: Enter in other mode — no action");
                }
            }
        }

        KeyEvent {
            modifiers: KeyModifiers::CONTROL,
            ..
        } => {
            tracing::debug!(key = ?input.code, "keyboard: unhandled Ctrl combo — ignored");
        }

        KeyEvent {
            code: KeyCode::Char(c),
            ..
        } => {
            tracing::debug!(ch = %c, mode = ?greeter.mode, "keyboard: Char — insert");
            insert_key(&mut greeter, c).await;
        }

        KeyEvent {
            code: KeyCode::Backspace,
            ..
        }
        | KeyEvent {
            code: KeyCode::Delete,
            ..
        } => {
            tracing::debug!(key = ?input.code, mode = ?greeter.mode, "keyboard: Backspace/Delete");
            delete_key(&mut greeter, input.code).await;
        }

        _ => {
            tracing::debug!(key = ?input.code, mods = ?input.modifiers, "keyboard: unmatched key — ignored");
        }
    }

    tracing::debug!(mode = ?greeter.mode, "keyboard::handle done");
    Ok(())
}

async fn insert_key(greeter: &mut Greeter, c: char) {
    let value = match greeter.mode {
        Mode::Username => &greeter.username.value,
        Mode::Password => &greeter.buffer,
        Mode::Command => &greeter.buffer,
        _ => {
            tracing::debug!("insert_key: mode has no buffer, skipping");
            return;
        }
    };

    let length = value.chars().count();
    let index = (length as i16 + greeter.cursor_offset) as usize;
    tracing::debug!(ch = %c, length, index, cursor_offset = greeter.cursor_offset, "insert_key");

    let left = value.chars().take(index);
    let right = value.chars().skip(index);

    let value = left.chain(vec![c]).chain(right).collect();
    let mode = greeter.mode;

    match mode {
        Mode::Username => greeter.username.value = value,
        Mode::Password => greeter.buffer = value,
        Mode::Command => greeter.buffer = value,
        _ => {}
    };
}

async fn delete_key(greeter: &mut Greeter, key: KeyCode) {
    let value = match greeter.mode {
        Mode::Username => &greeter.username.value,
        Mode::Password => &greeter.buffer,
        Mode::Command => &greeter.buffer,
        _ => {
            tracing::debug!("delete_key: mode has no buffer, skipping");
            return;
        }
    };

    let length = value.chars().count();
    let index = match key {
        KeyCode::Backspace => (length as i16 + greeter.cursor_offset - 1) as usize,
        KeyCode::Delete => (length as i16 + greeter.cursor_offset) as usize,
        _ => 0,
    };

    tracing::debug!(key = ?key, length, index, cursor_offset = greeter.cursor_offset, "delete_key");

    if value.chars().nth(index).is_some() {
        let left = value.chars().take(index);
        let right = value.chars().skip(index + 1);

        let value = left.chain(right).collect();

        match greeter.mode {
            Mode::Username => greeter.username.value = value,
            Mode::Password => greeter.buffer = value,
            Mode::Command => greeter.buffer = value,
            _ => return,
        };

        if let KeyCode::Delete = key {
            greeter.cursor_offset += 1;
        }
    } else {
        tracing::debug!(index, length, "delete_key: index out of range, nothing deleted");
    }
}

async fn validate_username(greeter: &mut Greeter, ipc: &Ipc) {
    tracing::info!(username_len = greeter.username.value.len(), "validate_username — sending CreateSession");
    greeter.working = true;
    greeter.message = None;

    ipc.send(Request::CreateSession {
        username: greeter.username.value.clone(),
    })
    .await;
    greeter.buffer = String::new();
}

#[cfg(test)]
mod tests;
