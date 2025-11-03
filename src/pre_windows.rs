use std::env::temp_dir;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::prelude::{Error, Result};

pub static PANDOC_PATH_UNPACK: OnceLock<PathBuf> = OnceLock::new();
pub const PANDOC_BINARY_EMBED: &[u8] = include_bytes!("../resources/pandoc_upx.exe");

/// Function to get the path to the unpacked pandoc binary
///
/// # Errors
/// If the pandoc binary could not be found or unpacked
/// or if the OS is not supported.
pub fn get_pandoc_path() -> Result<PathBuf> {
    let tmp_dir = temp_dir();
    let pandoc_name = "pandoc_upx.exe";

    // initial path creation
    let pandoc_path = tmp_dir.join(pandoc_name).to_path_buf();

    // initialize the OnceLock with the unpacked pandoc binary path
    unsafe { update_pandoc_unpacked(&pandoc_path) }

    let pandoc_path = PANDOC_PATH_UNPACK.get().ok_or_else(|| {
        Error::PandocNotFound("Could not find or unpack pandoc binary".to_string())
    })?;

    // swap out the static PANDOC_PATH_UNPACK with the new path
    // std::mem::replace(&mut unsafe .to_owned(), pandoc_path);
    dbg!(&pandoc_path);

    #[allow(clippy::needless_return)]
    return Ok(pandoc_path.to_owned());
}

/// Function to update the OnceLock with the unpacked pandoc binary path
/// if it does not already exist.
///
/// This function is unsafe because it modifies a static variable.
/// We can ensure safety by only calling this function once during initialization.
///
/// # Parameters
/// * `pandoc_path`: The path where the pandoc binary should be unpacked.
///
pub unsafe fn update_pandoc_unpacked(pandoc_path: &PathBuf) {
    PANDOC_PATH_UNPACK.get_or_init(|| {
        if !pandoc_path.exists() && cfg!(target_os = "windows") {
            let mut file = std::fs::File::create(&pandoc_path).expect("Could not create pandoc binary");
            file.write_all(PANDOC_BINARY_EMBED)
                .expect("Could not write pandoc binary");

            // Set permissions on created file for *nix systems
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

    PANDOC_PATH_UNPACK.get().unwrap();
}
