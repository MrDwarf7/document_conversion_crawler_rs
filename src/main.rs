mod cli;
mod conversion;
mod error;
mod lazy_logger;
mod prelude;

use std::sync::Arc;

use eyre::WrapErr;

pub use self::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::new();
    let level = args.verbosity_level().into();
    let _ = init_logger(level).init();

    let converter = Arc::new(conversion::pandoc::PandocConverter::new());

    let files = conversion::find_by_ext(&args.input_directory, &args.input_extension)?;
    info!("Found {} files to convert", files.len());

    if let Err(e) = conversion::convert_files(files, converter, &args.output_extension)
        .await
        .wrap_err("Failed to convert files")
    {
        error!("{:?}", e);
        return Err(Error::Generic(format!("Failed to convert files due to: {}", e)));
    }

    info!("Successfully converted all files");

    Ok(())
}
