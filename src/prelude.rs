// use std::env::temp_dir;
// use std::path::PathBuf;
// use std::sync::OnceLock;

// in-crate Error type
pub use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::EnvFilter;

pub use crate::error::Error;

// in-crate result type
pub type Result<T> = std::result::Result<T, Error>;

// Wrapper struct
#[allow(dead_code)]
pub struct W<T>(pub T);

pub fn time<T>(t: &str, f: impl FnOnce() -> T) -> T {
    eprintln!("{t}: Starting");
    let start = std::time::Instant::now();
    let r = f();
    let elapsed = start.elapsed();
    eprintln!("{t}: Elapsed: {elapsed:?}");
    r
}

pub type TracingSubscriber = tracing_subscriber::fmt::SubscriberBuilder<
    tracing_subscriber::fmt::format::DefaultFields,
    tracing_subscriber::fmt::format::Format<tracing_subscriber::fmt::format::Full>,
    tracing_subscriber::EnvFilter,
>;

pub fn init_logger(level: EnvFilter) -> TracingSubscriber {
    tracing_subscriber::fmt()
        .with_level(true)
        .with_ansi(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_env_filter(level)
    // .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
    // .with_timer(tracing_subscriber::fmt::time::SystemTime)
}

/// We don't include a binary for padnoc on unix-like systems
/// due to the ease of aquiring it via package managers etc.
/// we make a best-effort attempt to find pandoc in PATH or
/// via 'command -v pandoc'
#[rustfmt::skip]
#[cfg(not(target_os = "windows"))]
pub use crate::pre_unix::*;

/// We conditionally include the platform-specific prelude
/// Windows comes with an embedded pandoc binary that
/// requires unpacking and special treatment (because 'wInDows is aWEsOme')
#[rustfmt::skip]
#[cfg(target_os = "windows")]
pub use crate::pre_windows::*;
