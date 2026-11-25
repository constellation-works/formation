//! Single owner of Formation's stdout, stderr, and diagnostic rendering.

pub(crate) mod payload;

mod render;

pub(crate) use render::{failure, mail, summary};

pub(crate) fn error(error: &formation_core::Error) -> std::process::ExitCode {
    let status = failure(error);
    if matches!(error.code(), "room_required" | "identity_required") {
        std::process::ExitCode::from(2)
    } else {
        status
    }
}

pub(crate) fn init_logging() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .without_time()
        .with_max_level(tracing_subscriber::filter::LevelFilter::WARN)
        .try_init();
}
