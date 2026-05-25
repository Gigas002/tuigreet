use std::fs::OpenOptions;

use libratgreet::Greeter;
use tracing_appender::non_blocking::WorkerGuard;

pub fn init(greeter: &Greeter) -> Option<WorkerGuard> {
    use tracing_subscriber::filter::{LevelFilter, Targets};
    use tracing_subscriber::prelude::*;

    let logfile = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .clone();

    match (greeter.debug, logfile.open(&greeter.logfile)) {
        (true, Ok(file)) => {
            let (appender, guard) = tracing_appender::non_blocking(file);
            let target = Targets::new()
                .with_target("ratgreet", LevelFilter::DEBUG)
                .with_target("libratgreet", LevelFilter::DEBUG);

            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_writer(appender)
                        .with_line_number(true),
                )
                .with(target)
                .init();

            Some(guard)
        }

        _ => None,
    }
}
