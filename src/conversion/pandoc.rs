use std::io::Read;
use std::path::PathBuf;

use super::Converter;
use crate::pandoc_path;
use crate::prelude::*;

pub struct PandocConverter {
    name: PathBuf,
}

impl PandocConverter {
    /// Create a new PandocConverter
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
        let name = pandoc_path!();

        Self { name }
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
    async fn convert(&self, input: PathBuf, output: PathBuf) -> Result<()> {
        debug!("Converting '{}' to '{}'", input.display(), output.display());

        let filename = input.file_stem().unwrap().to_str().unwrap();
        let parent_folder = input.parent().unwrap();

        let media_folder = parent_folder.join(filename);

        let cmd = tokio::process::Command::new(&self.name)
            .arg("--extract-media")
            .arg(&media_folder)
            .arg("-s")
            .arg(&input)
            .arg("-o")
            .arg(&output)
            .output()
            .await;

        // println!("after cmd");

        let output = cmd.map_err(Error::from)?;

        if !output.status.success() {
            let mut stderr = String::new();
            output.stderr.as_slice().read_to_string(&mut stderr)?;
            return Err(Error::Generic(format!(
                "Failed to convert {} to {:?}: {}",
                input.display(),
                output,
                stderr
            )));
        }

        Ok(())
    }

    async fn check_installed(&self) -> crate::Result<bool> {
        let program_name = self.name.clone();
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
        .map_err(Error::from)?;

        trace!("Checked if {program_name_c:?} is installed: {checked:?}");

        if checked.is_err() {
            warn!("{program_name_c:?} is not installed");
            return Ok(false);
        };

        Ok(checked?)
    }

    #[inline]
    fn name(&self) -> impl AsRef<str> {
        self.name.display().to_string()
    }
}
