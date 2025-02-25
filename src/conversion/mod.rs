pub(crate) mod pandoc;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::task::JoinError;
use walkdir::WalkDir;

#[allow(unused_imports)]
use crate::lazy_logger::LazyLogger;
use crate::prelude::*;

const DANGER_CHARS: [&str; 2] = ["$", "~"];

#[async_trait::async_trait]
pub trait Converter {
    // TODO: Have convert take an 'engine' enum - pandoc, libreoffice, etc
    async fn convert(&self, input: PathBuf, output: PathBuf) -> Result<()>;
    async fn check_installed(&self) -> Result<bool>;
    fn name(&self) -> impl AsRef<str>;
}

pub type IndividualFiles = Vec<PathBuf>;
pub type TopLevelFolderNames = Vec<PathBuf>;

#[derive(Debug, Default)]
pub struct ProcessableEntities {
    pub individual_files:       IndividualFiles,
    pub top_level_folder_names: TopLevelFolderNames,
}

pub async fn find_by_ext(dir: &Path, ext: &str) -> Result<ProcessableEntities> {
    let mut collector = ProcessableEntities::default();

    debug!("Finding files with extension '{ext}' in '{dir:?}'");

    let ext = remove_dot(ext);
    let dir_len = dir.components().count();

    match fix_mangled_batch(dir) {
        Ok(_) => {}
        Err(e) => {
            error!("Error fixing mangled files: {:?}", e);
            return Err(e);
        }
    }

    match collect_batch(dir, dir_len, ext, &mut collector) {
        Ok(_) => {}
        Err(e) => {
            error!("Error collecting files: {:?}", e);
            return Err(e);
        }
    }

    debug!("Files: {:#?}", collector.individual_files);
    debug!("Found {} files with extension {}", collector.individual_files.len(), ext);

    Ok(collector)
}

fn collect_batch(dir: &Path, dir_len: usize, ext: &str, collector: &mut ProcessableEntities) -> Result<()> {
    for entry in WalkDir::new(dir).into_iter().flat_map(|e| e.ok()) {
        trace!("Checking {:?}", entry.path());

        if entry.path().is_dir() && entry.path().components().count() == dir_len + 1 {
            collector.top_level_folder_names.push(entry.path().to_path_buf());
        }

        if entry.path().is_file() && entry.path().extension().and_then(|s| s.to_str()) == Some(ext) {
            collector.individual_files.push(entry.path().to_path_buf());
        }
    }

    Ok(())
}

fn fix_mangled_batch(dir: &Path) -> Result<()> {
    for entry in WalkDir::new(dir).into_iter().flat_map(|e| e.ok()) {
        if !entry.path().to_str().unwrap().contains("~") || !entry.path().to_str().unwrap().contains("$") {
            continue;
        }

        warn!("Fixing file/folder containing '~' or '$' with '_' char: {:?}", entry.path());
        let fixed = fix_mangled_name(entry.path().to_str().unwrap());
        trace!("Original name: {:?} | Fixed name: {:?}", entry.path(), fixed);

        match std::fs::rename(entry.path(), &fixed) {
            Ok(_) => {
                debug!("Renamed file/folder to: {:?}", fixed);
            }
            Err(e) => {
                error!("Failed to rename file/folder: {:?}", e);
                return Err(Error::FailedRenameFile(entry.path().to_path_buf()));
            }
        }
    }
    Ok(())
}

#[inline]
fn fix_mangled_name(name: &str) -> String {
    let mut name = name.to_string();
    for c in &DANGER_CHARS {
        name = name.replace(c, "_");
    }
    name
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
    processable: ProcessableEntities,
    converter: Arc<C>,
    target_ext: &str,
) -> Result<()> {
    if let Ok(false) = converter.check_installed().await {
        return Err(Error::ConversionProgramNotInstalled(converter.name().as_ref().to_string()));
    }

    let converter = Arc::clone(&converter);

    // let mut p = LazyLogger::default();

    let tasks: Vec<_> = processable
        .individual_files
        .into_iter()
        .map(|file| {
            let output = file.with_extension(target_ext);
            let converter = Arc::clone(&converter);

            // p.log_input_output(&file, &ouput);

            tokio::task::spawn(async move {
                let fut = converter.convert(file, output);
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

pub async fn convert_files_with_output<C: Converter + Send + Sync + 'static>(
    processable: ProcessableEntities,
    converter: Arc<C>,
    target_ext: &str,
    output_dir: &Path,
) -> Result<()> {
    if let Ok(false) = converter.check_installed().await {
        return Err(Error::ConversionProgramNotInstalled(converter.name().as_ref().to_string()));
    }

    let converter = Arc::clone(&converter);

    // let mut p = LazyLogger::default();

    let tasks: Vec<_> = processable
        .individual_files
        .into_iter()
        .zip(processable.top_level_folder_names.into_iter())
        // BUG: If primary itter has many more
        // items, zipping will cause issues with how we
        // generate folder names for output
        //
        .map(|(input_file, top_level_name)| {
            // C:\temp\somefile.docx -> C:\temp\somefile.md

            let output_file_new_ext = input_file.with_extension(target_ext);

            let output = match top_level_generator(&top_level_name, output_dir) {
                Ok(output_dir) => {
                    let new_ext_name = match output_file_new_ext.file_name() {
                        Some(name) => name,
                        None => output_file_new_ext.as_os_str(),
                    };
                    let output_dir = output_dir.join(new_ext_name);

                    // replace any of '$' or '~' in any segment after the very first one (so we
                    // don't strip ~ == $HOME)
                    // remove_dangerous_chars(&output_dir)
                    output_dir
                }
                Err(e) => {
                    error!("Error creating output directory: {:?}", e);
                    output_dir.join(output_file_new_ext.file_name().unwrap())
                }
            };

            trace!("Output after corrections: {:?}", output);

            let converter = Arc::clone(&converter);

            // p.log_input_output(&file, &ouput);

            if output.exists() {
                warn!("Output file already exists: {:?}", output);
                return tokio::task::spawn(async move { Ok(()) });
            }

            tokio::task::spawn(async move {
                let fut = converter.convert(input_file, output);
                tokio::pin!(fut);
                (&mut fut).await
            })
        })
        .collect();

    // p.flush_async().await?;

    info!("Running conversion for {} files", tasks.len());

    let mut es_one = vec![];
    let mut es_two = vec![];

    let _ = totals(tasks, &mut es_one, &mut es_two).await;

    Ok(())
}

async fn totals(
    tasks: Vec<tokio::task::JoinHandle<Result<()>>>,
    es_one: &mut Vec<JoinError>,
    es_two: &mut Vec<Error>,
) {
    let task_len = tasks.len();

    for task in tasks {
        let task = task.await;

        let f_one = match task {
            Ok(o) => o,
            Err(e) => {
                es_one.push(e);
                continue;
            }
        };

        match f_one {
            Ok(_) => {}
            Err(e) => {
                es_two.push(e);
            }
        }
    }

    let total_errors = es_one.len() + es_two.len();
    let success = (task_len - total_errors) as f64;
    let perc = (success / task_len as f64) * 100.0;

    info!("Processes a total of: {} files", task_len);

    info!("Successly processed: {} files", success);
    info!("Failed a total of: {} files", total_errors);

    info!("Success rate: {:.2}%", perc);
}

fn top_level_generator(top_level_name: &Path, output_dir: &Path) -> Result<PathBuf> {
    let top_level_name_only = match top_level_name.components().next_back() {
        Some(name) => name.as_os_str().to_str().unwrap(),
        None => top_level_name.to_str().unwrap(),
    };

    let output_dir = output_dir.join(top_level_name_only);
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir).unwrap();
    }
    Ok(output_dir)
}

/// Check the path to ensure no 'DANGER_CHARS' are present in the path. If they are, replace them
/// with an underscore.
// fn remove_dangerous_chars(output_dir: &Path) -> PathBuf {
//     match output_dir.components().next_back() {
//         Some(component) => {
//             let component_str = component.as_os_str().to_str().unwrap();
//             let mut name = component_str.to_string();
//             for c in &DANGER_CHARS {
//                 name = name.replace(c, "_");
//             }
//             output_dir.with_file_name(name)
//         }
//         None => output_dir.to_path_buf(),
//     }
// }

#[cfg(test)]
mod conversion_tests {
    use super::*;

    #[tokio::test]
    async fn test_fix_mangled_name() {
        let name = "some~file$";
        let fixed = fix_mangled_name(name);
        assert_eq!(fixed, "some_file_");
    }
}
