use std::io::Read;
use std::path::{Path, PathBuf};

use crate::conversion::Converter;
use crate::pandoc_path;
use crate::prelude::*;

pub struct PandocConverter<P = PathBuf> {
    program_name: P,
}

impl PandocConverter {
    /// Create a new `PandocConverter`
    ///
    /// ### Note:
    /// This calls a macro `pandoc_path!` which is defined in `src/prelude.rs`
    ///
    /// The macro handles embedding the official pandoc binary into the executable
    /// and if it doesn't exist, it generates a temporary file and dumps the binary to it from the embed.
    ///
    /// If hash compare - this has been compressed using upx (`upx --best pandoc -o pandoc_upx`)
    #[inline]
    pub fn new() -> Self {
        let program_name = pandoc_path!();
        Self { program_name }
    }

    /// Creates a folder
    /// that follows the naming of
    /// `input_filename/media/stuff....`
    pub fn media_folder<P>(path: P) -> Result<impl AsRef<Path>>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let filename = path.file_stem().unwrap().to_str().ok_or_else(|| {
            Error::Generic(format!(
                "Failed to get file stem from path: {}",
                path.display()
            ))
        })?;
        let parent_folder = path.parent().ok_or_else(|| {
            Error::Generic(format!(
                "Failed to get parent folder from path: {:?}",
                path.display()
            ))
        })?;
        Ok(parent_folder.join(filename))
    }
}

impl Default for PandocConverter {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Converter for PandocConverter {
    async fn convert<P>(&self, input: P, output: P) -> Result<()>
    where
        P: AsRef<Path> + Send + Sync,
    {
        let input = input.as_ref();
        let output = output.as_ref();

        trace!("Converting '{}' to '{}'", input.display(), output.display());

        let media_folder = match PandocConverter::media_folder(&output) {
            Ok(folder) => folder,
            Err(e) => {
                warn!("Failed to create media folder: {e}");
                return Err(Error::MediaFolderCreationFailed(format!(
                    "Filename: {}, Parent: {}, Output: {}, Error: {e}",
                    input.file_stem().unwrap().display(),
                    input.parent().unwrap().display(),
                    output.display()
                )));
            }
        };
        let media_folder = media_folder.as_ref();

        trace!("Media folder: {:?}", media_folder);

        // let filename = input.file_stem().unwrap().to_str().unwrap();
        // let parent_folder = input.parent().unwrap();
        // let media_folder = parent_folder.join(filename);

        let cmd = tokio::process::Command::new(&self.program_name)
            .arg("--extract-media")
            .arg(media_folder)
            .arg("-s")
            .arg(input)
            .arg("-o")
            .arg(output)
            .output()
            .await;

        let output = cmd.map_err(Error::from)?;

        if !output.status.success() {
            let mut stderr = String::new();
            output.stderr.as_slice().read_to_string(&mut stderr)?;
            return Err(Error::Generic(format!(
                "Success checker: Failed to convert {} to {output:?}: {stderr}",
                input.display(),
            )));
        }

        Ok(())
    }

    async fn check_installed(&self) -> impl Into<bool> {
        let program_name = self.program_name.clone();
        let program_name_c = program_name.clone();

        let checked = tokio::task::spawn(async move {
            tokio::process::Command::new(program_name.clone())
                .arg("--version")
                .output()
                .await
                .map(|output| output.status.success())
                .map_err(Error::from)
        })
        .await
        .map_err(Error::from)
        .and_then(|res| res);

        debug!("Checked if {program_name_c:?} is installed: {checked:?}");

        if checked.is_err() {
            warn!("{program_name_c:?} is not installed");
            return false;
        }

        checked.unwrap_or(false)
    }

    #[inline]
    fn name(&self) -> impl AsRef<str> {
        self.program_name.display().to_string()
    }
}
