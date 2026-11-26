//! Tracing subscriber initialization for non-hook Formation commands.

pub(crate) fn init_logging() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .without_time()
        .with_max_level(tracing_subscriber::filter::LevelFilter::WARN)
        .try_init();
}
