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
    tracing::info!("app::run entered");

    register_panic_handler();
    tracing::debug!("panic handler registered");

    #[cfg(all(not(test), not(feature = "test-harness")))]
    {
        tracing::debug!("calling enable_raw_mode");
        enable_raw_mode()?;
        tracing::info!("enable_raw_mode OK");

        tracing::debug!("sending EnterAlternateScreen");
        execute!(io::stdout(), EnterAlternateScreen)?;
        tracing::info!("EnterAlternateScreen OK");
    }

    #[cfg(any(test, feature = "test-harness"))]
    tracing::info!("app::run: TEST/TEST-HARNESS mode — skipping raw mode and alternate screen");

    tracing::debug!("creating Terminal");
    let mut terminal = Terminal::new(backend)?;
    tracing::debug!("Terminal created");

    #[cfg(all(not(test), not(feature = "test-harness")))]
    {
        tracing::debug!("clearing terminal");
        terminal.clear()?;
        tracing::debug!("setting initial cursor position");
        terminal.set_cursor_position((0, 0))?;
        tracing::debug!("terminal initialized");
    }

    match terminal.size() {
        Ok(size) => tracing::info!(cols = size.width, rows = size.height, "terminal size"),
        Err(e) => tracing::warn!("could not get terminal size: {e}"),
    }

    tracing::debug!("creating IPC");
    let ipc = Ipc::new();
    tracing::debug!("IPC created");

    let greeter = Arc::new(RwLock::new(greeter));

    tracing::debug!("spawning IPC task");
    let ipc_task = tokio::task::spawn({
        let greeter = greeter.clone();
        let mut ipc = ipc.clone();

        async move {
            tracing::debug!("IPC task started");
            let mut ipc_loop_count: u64 = 0;
            loop {
                ipc_loop_count += 1;
                tracing::trace!(ipc_loop_count, "IPC task loop iteration");
                let result = ipc.handle(greeter.clone()).await;
                if let Err(e) = result {
                    tracing::warn!("IPC handle error: {e}");
                }
            }
        }
    });
    tracing::debug!("IPC task spawned");

    tracing::info!("entering main event loop");
    let mut event_loop_count: u64 = 0;

    loop {
        event_loop_count += 1;
        tracing::trace!(event_loop_count, "main loop iteration");

        let exit_status = greeter.read().await.exit;
        if let Some(status) = exit_status {
            tracing::info!("main loop: exit flag set, status={status}");

            ipc_task.abort();
            let _ = ipc_task.await;
            drop(greeter);

            return Err(status.into());
        }

        tracing::trace!("waiting for next event");
        let event = events.next().await;
        tracing::trace!(has_event = event.is_some(), "got event from channel");

        match event {
            Some(Event::Render) => {
                tracing::trace!("main loop: Render event");
                crate::ui::draw(greeter.clone(), &theme, &mut terminal).await?
            }

            Some(Event::Key(key)) => {
                tracing::debug!(key = ?key.code, mods = ?key.modifiers, "main loop: Key event");
                keyboard::handle(greeter.clone(), key, ipc.clone()).await?
            }

            Some(Event::Exit(status)) => {
                tracing::info!("main loop: Exit event status={status}");
                exit(&mut *greeter.write().await, status).await;
            }

            Some(Event::PowerCommand(command)) => {
                tracing::info!("main loop: PowerCommand event");
                if let PowerPostAction::ClearScreen =
                    libratgreet::power::run(&greeter, command).await
                {
                    tracing::info!("power: ClearScreen — cleaning up terminal");
                    execute!(io::stdout(), LeaveAlternateScreen)?;
                    terminal.set_cursor_position((1, 1))?;
                    terminal.clear()?;
                    disable_raw_mode()?;

                    ipc_task.abort();
                    let _ = ipc_task.await;
                    drop(greeter);

                    break;
                } else {
                    tracing::info!("power: Noop — staying in event loop");
                }
            }

            None => {
                tracing::warn!("main loop: event channel closed (None) — breaking");
                break;
            }
        }
    }

    tracing::info!("app::run returning Ok");
    Ok(())
}

pub async fn exit(greeter: &mut Greeter, status: AuthStatus) {
    tracing::info!("app::exit called with status={status}");

    match status {
        AuthStatus::Success => {
            tracing::info!("exit: Success — no IPC cancel needed");
        }
        AuthStatus::Cancel | AuthStatus::Failure => {
            tracing::info!("exit: Cancel/Failure — sending IPC cancel");
            Ipc::cancel(greeter).await;
        }
    }

    #[cfg(all(not(test), not(feature = "test-harness")))]
    {
        tracing::debug!("exit: calling clear_screen");
        clear_screen();
        tracing::debug!("exit: clear_screen done");
    }

    tracing::debug!("exit: leaving alternate screen and disabling raw mode");
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
    let _ = disable_raw_mode();

    greeter.exit = Some(status);
    tracing::info!("exit: greeter.exit set to {status}");
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
