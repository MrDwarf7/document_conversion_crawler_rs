use std::io::Read;
use std::path::PathBuf;

use super::Converter;
use crate::prelude::*;

pub struct PandocConverter {
    name: String,
}

impl PandocConverter {
    #[inline]
    pub fn new() -> Self {
        Self {
            name: "pandoc".into(),
        }
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
        // let input_c = input.clone();
        // let output_c = output.clone();

        debug!("Converting '{}' to '{}'", input.display(), output.display());

        let cmd = tokio::process::Command::new(&self.name)
            .arg("-s")
            .arg(&input)
            .arg("-o")
            .arg(&output)
            .output()
            .await;

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
        // let program_name = program_name.into();
        let program_name = self.name.clone();
        let program_name_c = program_name.clone();

        let checked = tokio::task::spawn(async move {
            // let program_name = program_name_c.clone();
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
        &self.name
    }
}
