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

        tracing::info!(frame_ms = (1000.0 / FRAME_RATE) as u64, "Events::new — spawning event task");

        tokio::task::spawn({
            let tx = tx.clone();

            async move {
                #[cfg(any(test, feature = "test-harness"))]
                {
                    tracing::info!("event task: TEST/TEST-HARNESS mode — timer-only, no keyboard");
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

                    tracing::info!("event task: PRODUCTION mode — EventStream + timer");

                    // How long to hold an ESC before deciding it is a real keypress
                    // rather than the leading byte of a fragmented VT escape sequence.
                    const ESC_DEBOUNCE: Duration = Duration::from_millis(50);

                    tracing::debug!("creating EventStream");
                    let mut stream = EventStream::new();
                    tracing::debug!("EventStream created OK");

                    let mut render_interval = tokio::time::interval(frame_duration);

                    let mut esc_pending: Option<(KeyEvent, Instant)> = None;
                    let mut consuming_vt_seq = false;
                    let mut loop_count: u64 = 0;

                    loop {
                        loop_count += 1;
                        tracing::trace!(loop_count, esc_pending = esc_pending.is_some(), consuming_vt_seq, "event loop top");

                        // Flush an expired pending ESC before blocking in select!.
                        if let Some((_, t)) = &esc_pending {
                            if t.elapsed() >= ESC_DEBOUNCE {
                                let (ev, _) = esc_pending.take().unwrap();
                                tracing::debug!("ESC timeout (pre-select) → emitting real ESC");
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
                                tracing::debug!("select: EventStream arm fired");
                                match &maybe_event {
                                    None => {
                                        tracing::warn!("EventStream returned None — stream ended");
                                        continue;
                                    }
                                    Some(Err(e)) => {
                                        tracing::warn!("EventStream error: {e}");
                                        continue;
                                    }
                                    Some(Ok(TermEvent::Key(k))) => {
                                        tracing::debug!(key = ?k.code, kind = ?k.kind, mods = ?k.modifiers, "EventStream: key event");
                                    }
                                    Some(Ok(other)) => {
                                        tracing::debug!(event = ?other, "EventStream: non-key event, skipping");
                                        continue;
                                    }
                                }

                                let Some(Ok(TermEvent::Key(key))) = maybe_event else {
                                    tracing::debug!("EventStream: pattern did not match after inner check — skipping");
                                    continue;
                                };
                                if key.kind != KeyEventKind::Press {
                                    tracing::debug!(key = ?key.code, kind = ?key.kind, "EventStream: non-Press event, skipping");
                                    continue;
                                }

                                tracing::debug!(key = ?key.code, mods = ?key.modifiers, "raw key event (passed filter)");

                                // State 1: absorbing continuation bytes of a VT sequence.
                                if consuming_vt_seq {
                                    tracing::debug!(key = ?key.code, "in consuming_vt_seq state");
                                    match key.code {
                                        KeyCode::Char(c)
                                            if c.is_ascii_uppercase() || c == '~' =>
                                        {
                                            tracing::debug!(ch = %c, "VT sequence end byte, done absorbing");
                                            consuming_vt_seq = false;
                                        }
                                        KeyCode::Char(c)
                                            if c.is_ascii_digit()
                                                || c == ';'
                                                || c == '[' =>
                                        {
                                            tracing::debug!(ch = %c, "VT sequence interior byte, absorbing");
                                        }
                                        _ => {
                                            tracing::debug!(key = ?key.code, "unexpected byte in VT sequence, passing through and leaving consume state");
                                            consuming_vt_seq = false;
                                            let _ = tx.send(Event::Key(key)).await;
                                            let _ = tx.send(Event::Render).await;
                                        }
                                    }
                                    continue;
                                }

                                // State 2: decide what follows the held ESC.
                                if let Some((held_esc, _)) = esc_pending.take() {
                                    tracing::debug!(next = ?key.code, "ESC pending, evaluating next key");
                                    match key.code {
                                        KeyCode::Char('[') | KeyCode::Char('O') => {
                                            tracing::debug!("ESC + CSI/SS3 introducer → entering consume state");
                                            consuming_vt_seq = true;
                                            continue;
                                        }
                                        KeyCode::Esc => {
                                            tracing::debug!("double ESC → emitting first, pending second");
                                            let _ = tx.send(Event::Key(held_esc)).await;
                                            let _ = tx.send(Event::Render).await;
                                            esc_pending = Some((key, Instant::now()));
                                            continue;
                                        }
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
                                    tracing::debug!(key = ?key.code, "emitting key to channel");
                                    let _ = tx.send(Event::Key(key)).await;
                                    let _ = tx.send(Event::Render).await;
                                }
                            }

                            _ = async {
                                match esc_remaining {
                                    Some(d) => tokio::time::sleep(d).await,
                                    None => std::future::pending::<()>().await,
                                }
                            } => {
                                tracing::debug!("select: ESC timeout arm fired");
                                if let Some((ev, _)) = esc_pending.take() {
                                    tracing::debug!("ESC timeout (select arm) → emitting real ESC");
                                    let _ = tx.send(Event::Key(ev)).await;
                                    let _ = tx.send(Event::Render).await;
                                }
                            }

                            _ = render_interval.tick() => {
                                tracing::trace!("select: render tick arm fired");
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
