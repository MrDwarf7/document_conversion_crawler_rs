#[allow(unused_imports)]
use crate::prelude::get_pandoc_path;

#[macro_export]
macro_rules! pandoc_path {
    () => {
        $crate::prelude::get_pandoc_path()
            .expect("Could not get internal/embedded pandoc path")
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
