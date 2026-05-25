use std::time::Duration;

use crossterm::event::KeyEvent;
use tokio::{
    process::Command,
    sync::mpsc::{self, Sender},
};

use crate::AuthStatus;

const FRAME_RATE: f64 = 2.0;

pub enum Event {
    Key(KeyEvent),
    Render,
    PowerCommand(Command),
    Exit(AuthStatus),
}

pub struct Events {
    rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,
}

impl Events {
    pub async fn new() -> Events {
        let (tx, rx) = mpsc::channel(10);
        let frame_duration = Duration::from_secs_f64(1.0 / FRAME_RATE);

        tokio::task::spawn({
            let tx = tx.clone();

            async move {
                #[cfg(any(test, feature = "test-harness"))]
                {
                    let mut render_interval = tokio::time::interval(frame_duration);
                    loop {
                        render_interval.tick().await;
                        let _ = tx.send(Event::Render).await;
                    }
                }

                #[cfg(all(not(test), not(feature = "test-harness")))]
                {
                    use std::time::Instant;
                    use crossterm::event::{Event as TermEvent, EventStream, KeyCode, KeyEventKind};
                    use futures_util::StreamExt as _;

                    // How long to hold an ESC before deciding it is a real keypress
                    // rather than the leading byte of a fragmented VT escape sequence.
                    const ESC_DEBOUNCE: Duration = Duration::from_millis(50);

                    let mut stream = EventStream::new();
                    let mut render_interval = tokio::time::interval(frame_duration);

                    // On real Linux VTs the kernel may deliver a function-key's escape
                    // sequence across multiple read() calls, so crossterm can emit
                    // KeyCode::Esc followed by individual chars like '[', '[', 'A'
                    // instead of the expected KeyCode::F(1).  We hold the ESC for up
                    // to ESC_DEBOUNCE ms; if the very next byte is '[' or 'O' we
                    // absorb the rest of the sequence instead of forwarding it.
                    let mut esc_pending: Option<(KeyEvent, Instant)> = None;
                    let mut consuming_vt_seq = false;

                    loop {
                        // Flush an expired pending ESC before blocking in select!.
                        if let Some((_, t)) = &esc_pending {
                            if t.elapsed() >= ESC_DEBOUNCE {
                                let (ev, _) = esc_pending.take().unwrap();
                                tracing::debug!("ESC timeout → emitting real ESC");
                                let _ = tx.send(Event::Key(ev)).await;
                                let _ = tx.send(Event::Render).await;
                            }
                        }

                        let esc_remaining = esc_pending.as_ref().and_then(|(_, t)| {
                            ESC_DEBOUNCE.checked_sub(t.elapsed())
                        });

                        tokio::select! {
                            biased;

                            maybe_event = stream.next() => {
                                let Some(Ok(TermEvent::Key(key))) = maybe_event else { continue };
                                if key.kind != KeyEventKind::Press { continue }

                                tracing::debug!(key = ?key.code, mods = ?key.modifiers, "raw key event");

                                // State 1: absorbing continuation bytes of a VT sequence.
                                if consuming_vt_seq {
                                    match key.code {
                                        // Uppercase letter or '~' ends the CSI sequence.
                                        KeyCode::Char(c)
                                            if c.is_ascii_uppercase() || c == '~' =>
                                        {
                                            tracing::debug!(ch = %c, "VT sequence end byte, done absorbing");
                                            consuming_vt_seq = false;
                                        }
                                        // Digits, ';', second '[' are interior bytes.
                                        KeyCode::Char(c)
                                            if c.is_ascii_digit()
                                                || c == ';'
                                                || c == '[' =>
                                        {
                                            tracing::debug!(ch = %c, "VT sequence interior byte, absorbing");
                                        }
                                        // Unexpected byte: stop consuming, pass through.
                                        _ => {
                                            tracing::debug!(key = ?key.code, "unexpected byte after VT sequence start, passing through");
                                            consuming_vt_seq = false;
                                            let _ = tx.send(Event::Key(key)).await;
                                            let _ = tx.send(Event::Render).await;
                                        }
                                    }
                                    continue;
                                }

                                // State 2: decide what follows the held ESC.
                                if let Some((held_esc, _)) = esc_pending.take() {
                                    match key.code {
                                        // '[' or 'O' = CSI / SS3 introducer → absorb.
                                        KeyCode::Char('[') | KeyCode::Char('O') => {
                                            tracing::debug!("ESC + CSI/SS3 introducer → absorbing VT sequence");
                                            consuming_vt_seq = true;
                                            continue;
                                        }
                                        // Another ESC: emit the held one, pend the new.
                                        KeyCode::Esc => {
                                            tracing::debug!("double ESC → emitting first, pending second");
                                            let _ = tx.send(Event::Key(held_esc)).await;
                                            let _ = tx.send(Event::Render).await;
                                            esc_pending = Some((key, Instant::now()));
                                            continue;
                                        }
                                        // Anything else: real ESC then another key.
                                        _ => {
                                            tracing::debug!(key = ?key.code, "ESC + other key → emitting both");
                                            let _ = tx.send(Event::Key(held_esc)).await;
                                            let _ = tx.send(Event::Key(key)).await;
                                            let _ = tx.send(Event::Render).await;
                                            continue;
                                        }
                                    }
                                }

                                // State 3: normal path.
                                if key.code == KeyCode::Esc {
                                    tracing::debug!("ESC received, holding for debounce");
                                    esc_pending = Some((key, Instant::now()));
                                } else {
                                    tracing::debug!(key = ?key.code, "emitting key");
                                    let _ = tx.send(Event::Key(key)).await;
                                    let _ = tx.send(Event::Render).await;
                                }
                            }

                            // ESC debounce timeout: the ESC was real, not a prefix.
                            _ = async {
                                match esc_remaining {
                                    Some(d) => tokio::time::sleep(d).await,
                                    None => std::future::pending::<()>().await,
                                }
                            } => {
                                if let Some((ev, _)) = esc_pending.take() {
                                    tracing::debug!("ESC timeout (select arm) → emitting real ESC");
                                    let _ = tx.send(Event::Key(ev)).await;
                                    let _ = tx.send(Event::Render).await;
                                }
                            }

                            _ = render_interval.tick() => {
                                let _ = tx.send(Event::Render).await;
                            }
                        }
                    }
                }
            }
        });

        Events { rx, tx }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }

    pub fn sender(&self) -> Sender<Event> {
        self.tx.clone()
    }
}
