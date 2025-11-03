use std::path::PathBuf;
use std::sync::OnceLock;

use tracing::{error, trace};

use crate::prelude::{Error, Result};

pub static PANDOC_PATH_UNPACK: OnceLock<PathBuf> = OnceLock::new();

/// Gets the path to the pandoc binary
/// This is platform dependent - the one you're calling
/// here is for unix-like systems.
/// If you're on windows, see `pre_windows.rs`
///
/// # Errors
/// * `Result::Err` - If the pandoc binary could not be found in PATH
///
pub fn get_pandoc_path() -> Result<PathBuf> {
    update_pandoc_unpacked(&PathBuf::new());
    let pandoc = PANDOC_PATH_UNPACK.get();
    match pandoc {
        Some(p) => Ok(p.to_owned()),
        None => {
            Err(Error::PandocNotFound("Could not find pandoc binary in PATH".to_string()))
        }
    }
}

/// Takes a best-effort scan of the PATH environment variable
/// If this cannot be done via PATH,
/// we fallback to an attempt to use 'command -v pandoc'.
/// If both fail, we exit the process with error code 1.
pub fn update_pandoc_unpacked(_pandoc_path: &PathBuf) {
    let maybe_res = scan_path_env_for_pandoc();

    if let Some(pandoc_path) = maybe_res.clone() {
        PANDOC_PATH_UNPACK.get_or_init(|| pandoc_path);
    }

    // we can try using the system native 'which' - not
    // hopeful, but it's last ditch effort to try and
    // do something reasonable
    if maybe_res.is_none() {
        trace!("Couldn't find pandoc in PATH, trying via 'command -v pandoc'");

        let cmd_res = std::process::Command::new("command")
            .arg("-v")
            .arg("pandoc")
            .output();

        if let Ok(cmd_output) = cmd_res {
            if cmd_output.status.success() {
                let stdout = String::from_utf8_lossy(&cmd_output.stdout);
                let pandoc_path = PathBuf::from(stdout.trim());
                trace!("Successfully found a pandoc binary via 'command -v pandoc'");

                PANDOC_PATH_UNPACK.get_or_init(|| pandoc_path);
            } else {
                trace!("'command -v pandoc' did not complete successfully");
                error!(
                    "'command -v pandoc' failed with status: {:?}",
                    cmd_output.status.code()
                );
                std::process::exit(1);
            }
        } else {
            error!("Failed to execute 'command -v pandoc' to find pandoc binary");
            std::process::exit(1);
        }
    }
}

/// Takes a best-effort scan of the PATH environment variable
/// initially uses predefined candidate names for pandoc binary
/// checking against each path entry in PATH.
///
#[must_use]
pub fn scan_path_env_for_pandoc() -> Option<PathBuf> {
    const CANDIDATES: [&str; 3] = ["pandoc", "pandoc-bin", "pandoc-cli"];

    let path_env = std::env::var("PATH").unwrap_or_default();

    for p in std::env::split_paths(&path_env) {
        for cand in &CANDIDATES {
            let full_path = p.join(cand);
            if full_path.exists() && full_path.is_file() {
                return Some(full_path);
            }
        }
    }

    None
}
