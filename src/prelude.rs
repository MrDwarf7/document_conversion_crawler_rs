use std::env::temp_dir;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

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
    eprintln!("{t}: Elapsed: {:?}", elapsed);
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

#[cfg(target_os = "windows")]
pub const PANDOC_BINARY_EMBED: &[u8] = include_bytes!("../resources/pandoc_upx.exe");

pub static PANDOC_PATH_UNPACK: OnceLock<PathBuf> = OnceLock::new();

pub fn get_pandoc_path() -> Result<PathBuf> {
    let tmp_dir = temp_dir();
    let pandoc_name = if cfg!(target_os = "windows") {
        "pandoc_upx.exe"
    } else {
        "pandoc_upx"
    };

    // initial path creation
    let pandoc_path = tmp_dir.join(pandoc_name).to_path_buf();
    // swap out the static PANDOC_PATH_UNPACK with the new path
    let pandoc_path =
        std::mem::replace(&mut unsafe { update_pandoc_unpacked(&pandoc_path) }.to_owned(), pandoc_path);

    dbg!(&pandoc_path);

    Ok(pandoc_path.to_owned())
}

unsafe fn update_pandoc_unpacked(pandoc_path: &PathBuf) -> &PathBuf {
    PANDOC_PATH_UNPACK.get_or_init(|| {
        if !pandoc_path.exists() {
            let mut file = File::create(&pandoc_path).expect("Could not create pandoc binary");
            file.write_all(PANDOC_BINARY_EMBED)
                .expect("Could not write pandoc binary");

            /// Set permissions on created file for *nix systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                // let mut perms = file.metadata()
                //     .expect("Could not get metadata for embeded pandoc binary - Linux is confused")
                //     .permissions();
                // perms.f
                // file.set_permissions(perms)?;

                std::fs::set_permissions(&pandoc_path, std::fs::Permissions::from_mode(0o755))
                    .expect("Could not set permissions for embeded pandoc binary - Linux is confused");
            }
        }
        pandoc_path.to_owned()
    });

    PANDOC_PATH_UNPACK.get().unwrap()
}

#[macro_export]
macro_rules! pandoc_path {
    () => {
        crate::prelude::get_pandoc_path().expect("Could not get internal/embedded pandoc path")
    };
}

#[macro_export]
macro_rules! crate_name {
    () => {
        env!("CARGO_PKG_NAME")
    };
}

#[macro_export]
macro_rules! crate_version {
    () => {
        env!("CARGO_PKG_VERSION")
    };
}

#[macro_export]
macro_rules! crate_description {
    () => {
        env!("CARGO_PKG_DESCRIPTION")
    };
}

#[macro_export]
macro_rules! crate_authors {
    ($sep:expr) => {{
        static AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
        if AUTHORS.contains(':') {
            static CACHED: std::sync::OnceLock<String> = std::sync::OnceLock::new();
            let s = CACHED.get_or_init(|| AUTHORS.replace(':', $sep));
            let s: &'static str = &*s;
            s
        } else {
            AUTHORS
        }
    }};
    () => {
        env!("CARGO_PKG_AUTHORS")
    };
}
