pub(crate) mod pandoc;

// use std::collections::HashMap;

use std::ops::{Div, Mul};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::task::JoinError;
use walkdir::WalkDir;

#[allow(unused_imports)]
use crate::lazy_logger::LazyLogger;
use crate::prelude::*;

#[async_trait::async_trait]
pub trait Converter {
    // TODO: Have convert take an 'engine' enum - pandoc, libreoffice, etc
    async fn convert<P: AsRef<Path> + Send + Sync>(
        &self,
        input: P,
        output: P,
    ) -> Result<()>;
    async fn check_installed(&self) -> impl Into<bool>;
    fn name(&self) -> impl AsRef<str>;
}

pub type IndividualFiles = Vec<PathBuf>;
pub type TopLevelFolderNames = Vec<PathBuf>;

#[derive(Debug, Default)]
pub struct ProcessableEntities {
    pub individual_files:       IndividualFiles,
    pub top_level_folder_names: TopLevelFolderNames,
}

impl AsMut<ProcessableEntities> for &mut ProcessableEntities {
    fn as_mut(&mut self) -> &mut ProcessableEntities {
        self
    }
}

pub async fn find_by_ext<S: AsRef<str>, P: AsRef<Path>>(dir: P, ext: S) -> Result<ProcessableEntities> {
    let mut collector = ProcessableEntities::default();
    let ext = ext.as_ref();
    let dir = dir.as_ref();
    let dir_path = dir.to_path_buf();

    debug!("Finding files with extension '{ext}' in '{dir:?}'");

    let ext = *remove_dot(Cow::Borrowed(&ext));
    trace!("Extension after removing dot: '{ext}'");
    let dir_len = dir.components().count();

    if let Err(e) = fix_mangled_batch(dir) {
        error!("Error fixing mangled files: {:?}", e);
        return Err(e);
    }

    let mut pe = ConvertableEnts::new_with_capacity(dir, files_to_process.len());

    // let root_depth = dir.components().count();
    for file_path in files_to_process {
        pe.add_file(file_path);
    }

    debug!("Files: {:#?}", collector.individual_files);
    debug!("Found {} files with extension {}", collector.individual_files.len(), ext);

    Ok(collector)
}

fn collect_batch<P, S, PE>(dir: P, dir_len: usize, ext: S, mut collector: PE) -> Result<()>
where
    P: AsRef<Path>,
    S: AsRef<str>,
    PE: AsMut<ProcessableEntities>,
{
    for entry in WalkDir::new(dir).into_iter().flat_map(|e| e.ok()) {
        trace!("Checking {:?}", entry.path());

        if entry.path().is_dir() && entry.path().components().count() == dir_len + 1 {
            collector
                .as_mut()
                .top_level_folder_names
                .push(entry.path().to_path_buf());
        }

        if path.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some(ext.as_ref())
        {
            to_process.push(path.to_path_buf());
        }
    }

    Ok(())
}

fn fix_mangled_batch<P: AsRef<Path>>(dir: P) -> Result<()> {
    for entry in WalkDir::new(dir).into_iter().flat_map(|e| e.ok()) {
        if !entry.path().to_str().unwrap().contains("~") || !entry.path().to_str().unwrap().contains("$") {
            continue;
        }

        warn!("Fixing file/folder containing '~' or '$' with '_' char: {:?}", entry.path());
        let fixed = fix_mangled_name(entry.path().to_str().unwrap())
            .as_ref()
            .to_string();
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
fn remove_dot(ext: &str) -> &str {
    ext.strip_prefix('.').unwrap_or(ext)
}

#[inline]
fn fix_mangled_name<S: AsRef<str>>(name: S) -> impl AsRef<str> + Into<String> {
    let mut name = name.as_ref().to_string();
    for c in &DANGER_CHARS {
        name = name.replace(c, "_");
    }
    name
}

#[inline]
fn remove_dot<'a>(ext: Cow<'a, &'a str>) -> Cow<'a, &'a str> {
    if ext.contains('.') {
        let i = ext.find('.').unwrap();
        return Cow::Owned(&ext[i + 1..]);
    }
    ext
}

pub async fn convert_files<C, S>(
    processable: ProcessableEntities,
    converter: Arc<C>,
    target_ext: S,
) -> Result<()>
where
    C: Converter + Send + Sync + 'static,
    S: AsRef<str>,
{
    if !converter.check_installed().await.into() {
        return Err(Error::ConversionProgramNotInstalled(converter.name().as_ref().to_string()));
    }

    // if let Ok(false) = converter.check_installed().await {
    //     return Err(Error::ConversionProgramNotInstalled(converter.name().as_ref().to_string()));
    // }

    let converter = Arc::clone(&converter);

    // let mut p = LazyLogger::default();

    let tasks: Vec<_> = processable
        .individual_files
        .into_iter()
        .map(|file| {
            let output = file.with_extension(target_ext.as_ref());
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

pub async fn convert_files_with_output<C, S, P>(
    processable: ProcessableEntities,
    converter: Arc<C>,
    target_ext: S,
    output_dir: P,
) -> Result<()>
where
    C: Converter + Send + Sync + 'static,
    S: AsRef<str>,
    P: AsRef<Path>,
{
    if !converter.check_installed().await.into() {
        return Err(Error::ConversionProgramNotInstalled(converter.name().as_ref().to_string()));
    }

    // if let Ok(false) = converter.check_installed().await {
    //     return Err(Error::ConversionProgramNotInstalled(converter.name().as_ref().to_string()));
    // }

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

            let output_file_new_ext = input_file.with_extension(target_ext.as_ref());

            let output = match top_level_generator(&top_level_name, &output_dir.as_ref().to_path_buf()) {
                Ok(output_dir) => {
                    let new_ext_name = match output_file_new_ext.file_name() {
                        Some(name) => name,
                        None => output_file_new_ext.as_os_str(),
                    };
                    let output_dir = output_dir.as_ref().join(new_ext_name);
                    fix_mangled_name(output_dir.to_str().unwrap()).as_ref().into()

                    // replace any of '$' or '~' in any segment after the very first one (so we
                    // don't strip ~ == $HOME)
                    // remove_dangerous_chars(&output_dir)
                    // output_dir
                }
                Err(e) => {
                    error!("Error creating output directory: {:?}", e);
                    output_dir.as_ref().join(output_file_new_ext.file_name().unwrap())
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

fn top_level_generator<P: AsRef<Path>>(top_level_name: P, output_dir: P) -> Result<impl AsRef<Path>> {
    let top_level_name_only = match top_level_name.as_ref().components().next_back() {
        Some(name) => name.as_os_str().to_str().unwrap(),
        None => top_level_name.as_ref().to_str().unwrap(),
    };

    trace!("Top level folder name only: {:?}", top_level_name_only);

    let output_dir = output_dir.as_ref().join(top_level_name_only);
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir).unwrap();
    }
    Ok(output_dir)
}

// Check the path to ensure no 'DANGER_CHARS' are present in the path. If they are, replace them
// with an underscore.
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
        let fixed = fix_mangled_name(name).as_ref().to_string();
        assert_eq!(fixed, "some_file_");
    }
}
