use std::{error::Error, io, sync::Arc};

use crossterm::{
    execute,
    terminal::{LeaveAlternateScreen, disable_raw_mode},
};
use libratgreet::{
    event::{Event, Events},
    greeter::{AuthStatus, Greeter},
    ipc::Ipc,
    keyboard,
    power::PowerPostAction,
};
use ratatui::{Terminal, backend::Backend};
use tokio::sync::RwLock;

#[cfg(all(not(test), not(feature = "test-harness")))]
use crossterm::terminal::{EnterAlternateScreen, enable_raw_mode};

use crate::ui::common::style::Theme;

mod greeter_init;

pub use greeter_init::init_greeter;

pub async fn run<B>(
    backend: B,
    greeter: Greeter,
    theme: Theme,
    mut events: Events,
) -> Result<(), Box<dyn Error>>
where
    B: Backend,
    <B as Backend>::Error: 'static,
{
    tracing::info!("ratgreet started");

    register_panic_handler();

    #[cfg(all(not(test), not(feature = "test-harness")))]
    {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
    }

    let mut terminal = Terminal::new(backend)?;

    #[cfg(all(not(test), not(feature = "test-harness")))]
    {
        terminal.clear()?;
        terminal.set_cursor_position((0, 0))?;
    }

    if let Ok(size) = terminal.size() {
        tracing::info!(cols = size.width, rows = size.height, "terminal size");
    }

    let ipc = Ipc::new();

    let greeter = Arc::new(RwLock::new(greeter));

    tokio::task::spawn({
        let greeter = greeter.clone();
        let mut ipc = ipc.clone();

        async move {
            loop {
                let _ = ipc.handle(greeter.clone()).await;
            }
        }
    });

    loop {
        if let Some(status) = greeter.read().await.exit {
            tracing::info!("exiting main loop");

            return Err(status.into());
        }

        match events.next().await {
            Some(Event::Render) => crate::ui::draw(greeter.clone(), &theme, &mut terminal).await?,
            Some(Event::Key(key)) => keyboard::handle(greeter.clone(), key, ipc.clone()).await?,

            Some(Event::Exit(status)) => {
                exit(&mut *greeter.write().await, status).await;
            }

            Some(Event::PowerCommand(command)) => {
                if let PowerPostAction::ClearScreen =
                    libratgreet::power::run(&greeter, command).await
                {
                    execute!(io::stdout(), LeaveAlternateScreen)?;
                    terminal.set_cursor_position((1, 1))?;
                    terminal.clear()?;
                    disable_raw_mode()?;

                    break;
                }
            }

            _ => {}
        }
    }

    Ok(())
}

pub async fn exit(greeter: &mut Greeter, status: AuthStatus) {
    tracing::info!("preparing exit with status {}", status);

    match status {
        AuthStatus::Success => {}
        AuthStatus::Cancel | AuthStatus::Failure => Ipc::cancel(greeter).await,
    }

    #[cfg(all(not(test), not(feature = "test-harness")))]
    clear_screen();

    let _ = execute!(io::stdout(), LeaveAlternateScreen);
    let _ = disable_raw_mode();

    greeter.exit = Some(status);
}

fn register_panic_handler() {
    let hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        #[cfg(all(not(test), not(feature = "test-harness")))]
        clear_screen();

        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();

        hook(info);
    }));
}

#[cfg(all(not(test), not(feature = "test-harness")))]
fn clear_screen() {
    use ratatui::backend::CrosstermBackend;

    let backend = CrosstermBackend::new(io::stdout());

    if let Ok(mut terminal) = Terminal::new(backend) {
        let _ = terminal.hide_cursor();
        let _ = terminal.clear();
    }
}
