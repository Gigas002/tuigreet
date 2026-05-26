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
                    use crossterm::event::{Event as TermEvent, EventStream, KeyEventKind};
                    use futures_util::StreamExt as _;

                    tracing::info!("event task: PRODUCTION mode — EventStream + timer");

                    // Crossterm already parses Linux-VT sequences (e.g. ESC [ [ A → F1).
                    // Do not intercept ESC or CSI bytes here: debouncing/swallowing breaks
                    // function keys and leaks '[' into the input buffer on greetd's VT.
                    async fn drain_pending_keys() -> Vec<KeyEvent> {
                        use crossterm::event::{self, KeyEventKind};

                        let mut keys = Vec::new();
                        while event::poll(Duration::ZERO).unwrap_or(false) {
                            match event::read() {
                                Ok(TermEvent::Key(key)) if key.kind == KeyEventKind::Press => {
                                    keys.push(key);
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    tracing::warn!("event::read while draining: {e}");
                                    break;
                                }
                            }
                        }
                        keys
                    }

                    async fn emit_keys(tx: &mpsc::Sender<Event>, keys: Vec<KeyEvent>) {
                        for key in keys {
                            tracing::debug!(key = ?key.code, mods = ?key.modifiers, "emitting key");
                            let _ = tx.send(Event::Key(key)).await;
                            let _ = tx.send(Event::Render).await;
                        }
                    }

                    tracing::debug!("creating EventStream");
                    let mut stream = EventStream::new();
                    tracing::debug!("EventStream created OK");

                    let mut render_interval = tokio::time::interval(frame_duration);

                    loop {
                        tokio::select! {
                            biased;

                            maybe_event = stream.next() => {
                                match maybe_event {
                                    None => {
                                        tracing::warn!("EventStream returned None — stream ended");
                                    }
                                    Some(Err(e)) => {
                                        tracing::warn!("EventStream error: {e}");
                                    }
                                    Some(Ok(TermEvent::Key(key))) if key.kind == KeyEventKind::Press => {
                                        tracing::debug!(key = ?key.code, mods = ?key.modifiers, "EventStream: key");
                                        let mut keys = vec![key];
                                        keys.extend(drain_pending_keys().await);
                                        emit_keys(&tx, keys).await;
                                    }
                                    Some(Ok(other)) => {
                                        tracing::debug!(event = ?other, "EventStream: non-key event, skipping");
                                    }
                                }
                            }

                            _ = render_interval.tick() => {
                                tracing::trace!("select: render tick");
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
