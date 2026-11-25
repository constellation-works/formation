//! Single owner of Formation's stdout, stderr, and diagnostic rendering.

pub(crate) mod payload;

mod render;

pub(crate) use render::{
    HistoryFormat, failure, hook_error, hook_json, hook_rewake, hook_usage, mail, sent_history,
    summary,
};

pub(crate) fn error(error: &formation_core::Error) -> std::process::ExitCode {
    render::mail_error(&payload::error(error))
}

pub(crate) fn init_logging() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .without_time()
        .with_max_level(tracing_subscriber::filter::LevelFilter::WARN)
        .try_init();
}
