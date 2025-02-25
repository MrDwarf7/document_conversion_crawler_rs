use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Generic error handler: {0}")]
    Generic(String),

    #[error("Tokio spawn/join error: {0}")]
    TokioJoinError(#[from] tokio::task::JoinError),

    #[error("Tokio IO error: {0}")]
    TokioIoError(#[from] tokio::io::Error),

    #[error("Directory traversal error: {0}")]
    DirectoryTraversalError(#[from] walkdir::Error),

    #[error("Pandoc conversion error, failed for: {0}")]
    PandocConversionError(String),

    #[error("Invalid extension: {0}")]
    InvalidExtension(String),

    #[error("Pandoc is not installed")]
    PandocNotInstalled,

    #[error("Conversion program not installed: {0}")]
    ConversionProgramNotInstalled(String),

    #[error("Media folder creation failed: context: {0}")]
    MediaFolderCreationFailed(String),

    #[error("Failed to rename file: {0}")]
    FailedRenameFile(PathBuf),
}
