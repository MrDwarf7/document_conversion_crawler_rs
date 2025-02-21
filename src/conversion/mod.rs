pub(crate) mod pandoc;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use walkdir::WalkDir;

#[allow(unused_imports)]
use crate::lazy_logger::LazyLogger;
use crate::prelude::*;

#[async_trait::async_trait]
pub trait Converter {
    // TODO: Have convert take an 'engine' enum - pandoc, libreoffice, etc
    async fn convert(&self, input: PathBuf, output: PathBuf) -> Result<()>;
    async fn check_installed(&self) -> Result<bool>;
    fn name(&self) -> impl AsRef<str>;
}

pub fn find_by_ext(dir: &Path, ext: &str) -> Result<Vec<PathBuf>> {
    let mut files = vec![];

    debug!("Finding files with extension '{ext}' in '{dir:?}'");

    let ext = remove_dot(ext);

    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        debug!("Checking {:?}", entry.path());

        if entry.path().starts_with("~") || entry.path().starts_with("$") {
            continue;
        }

        if entry.path().is_file() && entry.path().extension().and_then(|s| s.to_str()) == Some(ext) {
            files.push(entry.path().to_path_buf());
        }
    }

    debug!("Found {} files with extension {}", files.len(), ext);
    debug!("Files: {:#?}", files);

    Ok(files)
}

#[inline]
fn remove_dot(ext: &str) -> &str {
    if ext.contains('.') {
        let i = ext.find('.').unwrap();
        &ext[i + 1..]
    } else {
        ext
    }
}

pub async fn convert_files<C: Converter + Send + Sync + 'static>(
    files: Vec<PathBuf>,
    converter: Arc<C>,
    target_ext: &str,
) -> Result<()> {
    if let Ok(false) = converter.check_installed().await {
        return Err(Error::ConversionProgramNotInstalled(converter.name().as_ref().to_string()));
    }

    let converter = Arc::clone(&converter);

    // let mut p = LazyLogger::default();

    let tasks: Vec<_> = files
        .into_iter()
        .map(|file| {
            let ouput = file.with_extension(target_ext);
            let converter = Arc::clone(&converter);

            // p.log_input_output(&file, &ouput);

            tokio::task::spawn(async move {
                let fut = converter.convert(file, ouput);
                tokio::pin!(fut);
                (&mut fut).await
            })
        })
        .collect();

    // p.flush_async().await?;

    info!("Running conversion for {} files", tasks.len());

    for task in tasks {
        let task = task.await;
        task.map_err(Error::from)??;
    }

    Ok(())
}
