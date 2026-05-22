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
                loop {
                    if crossterm::event::poll(frame_duration).unwrap_or(false) {
                        while crossterm::event::poll(Duration::ZERO).unwrap_or(false) {
                            if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read()
                            {
                                let _ = tx.send(Event::Key(key)).await;
                            }
                        }
                    }
                    let _ = tx.send(Event::Render).await;
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
