use std::{io, process};

use clap::Parser;
use libtuigreet::{event::Events, greeter::AuthStatus};
use tui::backend::CrosstermBackend;
use tuigreet::{app, cli::Cli, settings};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let settings = match settings::Settings::load(&(&cli).into()) {
        Ok(settings) => settings,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1);
        }
    };

    let theme = settings.theme.clone();
    let backend = CrosstermBackend::new(io::stdout());
    let events = Events::new().await;
    let greeter = settings::init_greeter(events.sender(), &settings).await;

    if let Err(error) = app::run(backend, greeter, theme, events).await {
        if let Some(AuthStatus::Success) = error.downcast_ref::<AuthStatus>() {
            return;
        }

        process::exit(1);
    }
}
