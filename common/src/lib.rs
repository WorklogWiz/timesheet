use std::env;
use std::fs::File;
use env_logger::{Env, Logger};
use log::{debug, Level, Log};

pub mod config;
pub mod journal;
pub mod date;

use log;

pub fn configure_logging(log_level: log::Level) {
    let mut tmp_dir = env::temp_dir();
    tmp_dir.push("jira_worklog.log");


    let target = Box::new(File::create(tmp_dir).expect("Can't create file"));

    // If nothing else was specified in RUST_LOG, use 'warn'
    env_logger::Builder::from_env(Env::default().default_filter_or(match log_level {
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
            Level::Trace => "trace"
    })
    )
        // .target(env_logger::Target::Pipe(target))
        .init();
    debug!("Logging started");
}
