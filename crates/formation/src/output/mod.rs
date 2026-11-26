//! Single owner of Formation's stdout, stderr, and diagnostic rendering.

mod hook;

mod mail;

mod roster;

mod shared;

mod watch;

pub(crate) use hook::{
    hook_error, hook_json, hook_rewake, hook_usage, prompt as hook_prompt, stop as hook_stop,
};

pub(crate) use mail::{HistoryFormat, payload as mail_payload, sent_history, write as mail_write};

pub(crate) use roster::summary;

pub(crate) use shared::failure;

pub(crate) use watch::{
    exited as watch_exited, list as watch_list, payload as watch_payload,
    set_payload as watch_set_payload, write as watch_write,
};

pub(crate) fn error(error: &formation_core::Error) -> std::process::ExitCode {
    shared::write_json_error(&shared::error_payload(error))
}

pub(crate) fn init_logging() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .without_time()
        .with_max_level(tracing_subscriber::filter::LevelFilter::WARN)
        .try_init();
}
