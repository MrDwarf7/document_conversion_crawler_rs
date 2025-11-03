mod cli;
mod conversion;
mod error;
mod lazy_logger;
mod macros;
mod prelude;

// platform-specific prelude setup
#[cfg(unix)]
mod pre_unix;

// platform-specific prelude setup
#[cfg(windows)]
mod pre_windows;

use std::sync::Arc;

pub use crate::prelude::*;

// perhaps we use channels to send/recv. The Command output into a bytes channel buffer
// -- AIM: smooth out the programm calls to pandoc binary (beofre using a native rs lib)

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::new();
    let level = args.verbosity_level().into();
    init_logger(level).init();

    let converter = Arc::new(conversion::pandoc::PandocConverter::new());

    let convertables = conversion::find_by_ext(
        //
        &args.input_directory,
        &args.input_extension,
    )
    .await?;

    info!("Found {} files to convert", convertables.as_ref().count());
    // trace!("Processable Entities: {:#?}", processable);

    if let Some(ref output_dir) = args.output_directory
        && !output_dir.exists()
    {
        tokio::fs::create_dir_all(output_dir).await?;
    }

    conversion::convert_files(
        convertables,
        converter,
        &args.output_extension.as_str(),
        args.output_directory.as_ref(),
    )
    .await?;

    info!("Successfully converted all files");

    Ok(())
}
