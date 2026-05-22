mod backend;
mod output;

use std::{
    panic,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use libgreetd_stub::SessionOptions;
use ratatui::buffer::Buffer;
use tempfile::NamedTempFile;
use tokio::{
    sync::{
        RwLock,
        mpsc::{Receiver, Sender},
    },
    task::{JoinError, JoinHandle},
};

use libratgreet::{
    event::{Event, Events},
    greeter::Greeter,
    model::sessions::SessionSource,
};
use ratgreet::{
    app::{self, init_greeter},
    settings::{CliOverrides, Settings},
};

pub(super) use self::{
    backend::{TestBackend, output},
    output::*,
};

pub(super) struct IntegrationRunner(Arc<RwLock<_IntegrationRunner>>);

struct _IntegrationRunner {
    server: Option<JoinHandle<()>>,
    client: Option<JoinHandle<()>>,

    pub buffer: Arc<Mutex<Buffer>>,
    pub sender: Sender<Event>,
    pub tick: Receiver<bool>,
}

impl Clone for IntegrationRunner {
    fn clone(&self) -> Self {
        IntegrationRunner(Arc::clone(&self.0))
    }
}

impl IntegrationRunner {
    pub async fn new(opts: SessionOptions, builder: Option<fn(&mut Greeter)>) -> IntegrationRunner {
        Self::new_with_config(opts, builder, None, None, (200, 40)).await
    }

    pub async fn new_with_size(
        opts: SessionOptions,
        builder: Option<fn(&mut Greeter)>,
        size: (u16, u16),
    ) -> IntegrationRunner {
        Self::new_with_config(opts, builder, None, None, size).await
    }

    pub async fn new_with_config(
        opts: SessionOptions,
        builder: Option<fn(&mut Greeter)>,
        config: Option<PathBuf>,
        theme: Option<PathBuf>,
        size: (u16, u16),
    ) -> IntegrationRunner {
        let socket = NamedTempFile::new().unwrap().into_temp_path().to_path_buf();

        let (backend, buffer, tick) = TestBackend::new(size.0, size.1);
        let events = Events::new().await;
        let sender = events.sender();

        let server = tokio::task::spawn({
            let socket = socket.clone();

            async move {
                libgreetd_stub::start(&socket, &opts).await;
            }
        });

        let settings = Settings::load(&CliOverrides {
            config,
            theme,
            ..CliOverrides::default()
        })
        .expect("integration config must load");
        let theme = settings.theme.clone();

        let client = tokio::task::spawn(async move {
            let mut greeter = init_greeter(events.sender(), &settings).await;
            greeter.session_source = SessionSource::Command("uname".to_string());

            if let Some(builder) = builder {
                builder(&mut greeter);
            }

            greeter.logfile = "/tmp/ratgreet.log".to_string();
            greeter.socket = socket.to_str().unwrap().to_string();
            greeter.events = Some(events.sender());
            greeter.connect().await;

            let _ = app::run(backend, greeter, theme, events).await;
        });

        IntegrationRunner(Arc::new(RwLock::new(_IntegrationRunner {
            server: Some(server),
            client: Some(client),
            buffer,
            sender,
            tick,
        })))
    }

    pub async fn join_until_client_exit(&mut self, mut events: JoinHandle<()>) {
        let (mut server, mut client) = {
            let mut runner = self.0.write().await;

            (runner.server.take().unwrap(), runner.client.take().unwrap())
        };

        let mut exited = false;

        while !exited {
            tokio::select! {
              _ = tokio::time::sleep(Duration::from_secs(5)) => break,
              _ = (&mut server) => {}
              _ = (&mut client) => { exited = true; },
              ret = (&mut events), if !events.is_finished() => rethrow(ret),
            }
        }

        assert!(exited, "ratgreet did not exit");
    }

    pub async fn join_until_end(&mut self, events: JoinHandle<()>) {
        let (server, client) = {
            let mut runner = self.0.write().await;

            (runner.server.take().unwrap(), runner.client.take().unwrap())
        };

        tokio::select! {
          _ = tokio::time::sleep(Duration::from_secs(5)) => {},
          _ = server => {}
          _ = client => {},
          ret = events => rethrow(ret),
        }
    }

    #[allow(unused)]
    pub async fn wait_until_buffer_contains(&mut self, needle: &str) {
        loop {
            if output(&self.0.read().await.buffer).contains(needle) {
                return;
            }

            self.wait_for_render().await;
        }
    }

    #[allow(unused, unused_must_use)]
    pub async fn send_key(&self, key: KeyCode) {
        self.0
            .write()
            .await
            .sender
            .send(Event::Key(KeyEvent::new(key, KeyModifiers::empty())))
            .await;
    }

    #[allow(unused, unused_must_use)]
    pub async fn send_modified_key(&self, key: KeyCode, modifiers: KeyModifiers) {
        self.0
            .write()
            .await
            .sender
            .send(Event::Key(KeyEvent::new(key, modifiers)))
            .await;
    }

    #[allow(unused, unused_must_use)]
    pub async fn send_text(&self, text: &str) {
        for char in text.chars() {
            self.0
                .write()
                .await
                .sender
                .send(Event::Key(KeyEvent::new(
                    KeyCode::Char(char),
                    KeyModifiers::empty(),
                )))
                .await;
        }

        self.0
            .write()
            .await
            .sender
            .send(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::empty(),
            )))
            .await;
    }

    #[allow(unused)]
    pub async fn wait_for_render(&mut self) {
        self.0.write().await.tick.recv().await;
    }

    pub async fn output(&self) -> Output {
        Output(output(&self.0.read().await.buffer))
    }
}

fn rethrow(result: Result<(), JoinError>) {
    if let Err(err) = result
        && let Ok(panick) = err.try_into_panic()
    {
        panic::resume_unwind(panick);
    }
}
