pub(crate) mod pandoc;

// use std::collections::HashMap;

use std::ops::{Div, Mul};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use walkdir::WalkDir;

#[allow(unused_imports)]
use crate::lazy_logger::LazyLogger;
use crate::prelude::*;

const DANGER_CHARS: [&str; 2] = ["$", "~"];

static INITIAL_CAPACITY: usize = 1024;

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

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub abs_path: PathBuf,
    pub rel_path: PathBuf,
    pub depth:    usize,
}

impl FileEntry {
    pub fn new<P: AsRef<Path>>(abs_path: P, rel_path: P, depth: usize) -> Self {
        Self {
            abs_path: abs_path.as_ref().to_path_buf(),
            rel_path: rel_path.as_ref().to_path_buf(),
            depth,
        }
    }
}

impl AsRef<FileEntry> for FileEntry {
    fn as_ref(&self) -> &FileEntry {
        self
    }
}

#[derive(Debug, Default)]
pub struct ConvertableEnts<P: AsRef<Path> = PathBuf, F: AsRef<FileEntry> = FileEntry> {
    pub input_root: P,
    pub files:      Vec<F>,
    // /// usize -> index of self.files
    // pub files_by_parent: HashMap<PathBuf, Vec<usize>>,
}

impl AsRef<ConvertableEnts> for ConvertableEnts {
    fn as_ref(&self) -> &ConvertableEnts {
        self
    }
}

impl AsMut<ConvertableEnts> for &mut ConvertableEnts {
    fn as_mut(&mut self) -> &mut ConvertableEnts {
        self
    }
}

impl ConvertableEnts {
    pub fn new_with_capacity<P: AsRef<Path>>(root: P, cap: usize) -> Self {
        Self {
            input_root: root.as_ref().to_path_buf(),
            files:      Vec::with_capacity(cap),
            // files_by_parent: HashMap::new(),
        }
    }

    pub fn add_file<P: AsRef<Path>>(&mut self, abs_path: P) {
        let abs_path = abs_path.as_ref();
        let relative = abs_path.strip_prefix(&self.input_root).unwrap();
        let root_dir_depth = self.input_root.components().count();

        let depth = abs_path.components().count() - root_dir_depth;

        // let parent = abs_path.parent().unwrap().to_path_buf();
        // let idx = self.files.len();

        self.files.push(FileEntry::new(abs_path, relative, depth));
        // self.files_by_parent.entry(parent).or_default().push(idx);
    }

    pub fn count(&self) -> usize {
        self.files.len()
    }
}

pub async fn convert_files<Ce, C, S, P>(
    convertables: Ce,
    converter: Arc<C>,
    target_ext: S,
    output_dir: Option<P>,
) -> Result<()>
where
    Ce: AsRef<ConvertableEnts>,
    C: Converter + Send + Sync + 'static,
    S: AsRef<str>,
    P: AsRef<Path>,
{
    if !converter.check_installed().await.into() {
        return Err(Error::ConversionProgramNotInstalled(
            converter.name().as_ref().to_string(),
        ));
    }

    let convertables = convertables.as_ref();
    let mut tasks = Vec::with_capacity(convertables.count());

    for entry in &convertables.files {
        let input = &entry.abs_path;

        let output = if let Some(ref out_dir) = output_dir {
            let rel_with_new_ext = entry.rel_path.with_extension(target_ext.as_ref());
            let output = out_dir.as_ref().join(rel_with_new_ext);

            if let Some(parent) = output.parent()
                && !parent.exists()
            {
                tokio::fs::create_dir_all(parent).await?;
            }
            output
        } else {
            input.with_extension(target_ext.as_ref())
        };

        if output.exists() {
            warn!("Output file already exists: {output:?}");
            continue;
        }

        let converter = Arc::clone(&converter);
        let input = input.clone();

        tasks.push(tokio::spawn(async move { converter.convert(input, output).await }));
    }

    info!("Running conversion for {} files", tasks.len());

    let (success, failed) = totals(tasks).await;
    info!("Successly processed: {success} files");

    if failed > 0 {
        warn!("Conversion completed with {failed} errors.");
    }

    let total = success + failed;
    let success_perc = success.div(total).mul(100);
    info!("Overall success rate: {success_perc:.2}%");

    Ok(())
}

pub async fn find_by_ext<S, P>(dir: P, ext: S) -> Result<ConvertableEnts>
where
    S: AsRef<str>,
    P: AsRef<Path>,
{
    let ext = ext.as_ref();
    let dir = dir.as_ref();
    let dir_path = dir.to_path_buf();

    debug!("Finding files with extension '{ext}' in '{dir:?}'");

    let ext = remove_dot(ext).to_string();
    debug!("Extension after removing dot: '{ext}'");

    // let dir_len = dir.components().count(); // original

    let ext_clone = ext.clone();
    let (files_to_fix, files_to_process) =
        tokio::task::spawn_blocking(move || discover_and_cat(dir_path, ext_clone))
            .await?;

    if !files_to_fix.is_empty() {
        fix_mangled_par(files_to_fix).await?;
    }

    let mut pe = ConvertableEnts::new_with_capacity(dir, files_to_process.len());

    // let root_depth = dir.components().count();
    for file_path in files_to_process {
        pe.add_file(file_path);
    }

    let l = pe.count();
    debug!("Found {l} files with extension '{ext}'");

    Ok(pe)
}

fn discover_and_cat<S: AsRef<str>, P: AsRef<Path>>(
    dir: P,
    ext: S,
) -> (Vec<impl AsRef<Path>>, Vec<impl AsRef<Path>>) {
    let mut to_fix = vec![];
    let mut to_process = Vec::with_capacity(INITIAL_CAPACITY);

    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let path = entry.path();

        if needs_fixing(path) {
            to_fix.push(path.to_path_buf());
        }

        if path.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some(ext.as_ref())
        {
            to_process.push(path.to_path_buf());
        }
    }

    (to_fix, to_process)
}

#[inline]
fn needs_fixing<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref()
        .to_str()
        .is_some_and(|s| DANGER_CHARS.iter().any(|&c| s.contains(c)))

    // .map(|s| DANGER_CHARS.iter().any(|&c| s.contains(c)))
    // .unwrap_or(false)
}

async fn fix_mangled_par<P: AsRef<Path>>(paths: Vec<P>) -> Result<()> {
    let tasks: Vec<_> = paths
        .into_iter()
        .map(|path| {
            let path = path.as_ref().to_path_buf();
            tokio::spawn(async move { fix_single_file(path).await })
        })
        .collect();

    for task in tasks {
        task.await??;
    }

    Ok(())
}

async fn fix_single_file<P: AsRef<Path>>(path: P) -> Result<()> {
    let path_str = path
        .as_ref()
        .to_str()
        .ok_or_else(|| Error::Generic("Invalid UTF-8 in path".to_string()))?;

    let fixed = fix_mangled_name(path_str).as_ref().to_string();

    warn!("Fixing file/folder: {:?} -> {fixed}", path.as_ref().display());

    tokio::fs::rename(&path, &fixed).await.map_err(|e| {
        error!("Failed to rename: {e:?}");
        Error::FailedRenameFile(path.as_ref().to_path_buf())
    })?;

    debug!("Renamed file/folder to: {fixed}");
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

type SuccessCount = usize;
type FailedCount = usize;
type TotalsResult = (SuccessCount, FailedCount);

async fn totals(tasks: Vec<tokio::task::JoinHandle<Result<()>>>) -> TotalsResult {
    // let task_len: f64 = tasks.len() as f64;
    // let mut errors = vec![];

    let mut success: usize = 0;
    let mut failed: usize = 0;

    for task in tasks {
        match task.await {
            Ok(Ok(())) => success += 1,
            Ok(Err(e)) => {
                error!("Task failed with error: {:?}", e);
                failed += 1;
            }
            Err(e) => {
                error!("Task panicked or was cancelled: {:?}", e);
                failed += 1;
            }
        }
    }
    (success, failed)
}

#[cfg(test)]
mod conversion_tests {
    use super::*;

    #[tokio::test]
    async fn test_fix_mangled_name() {
        let name = "some~file$";
        let fixed = fix_mangled_name(name).as_ref().to_string();
        assert_eq!(fixed, "some_file_");
    }

    #[test]
    fn test_remove_dot() {
        assert_eq!(remove_dot(".md"), "md");
        assert_eq!(remove_dot("md"), "md");
        assert_eq!(remove_dot(".tar.gz"), "tar.gz");
    }

    #[test]
    fn test_needs_fixing() {
        assert!(needs_fixing(Path::new("file~.txt")));
        assert!(needs_fixing(Path::new("$file.txt")));
        assert!(!needs_fixing(Path::new("normal.txt")));
    }
}
