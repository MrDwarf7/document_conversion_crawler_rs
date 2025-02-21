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

    // perhaps we use channels to send/recv. The Command output into a bytes channel buffer
    // -- AIM: smooth out the programm calls to pandoc binary (beofre using a native rs lib)

    let converted_task = tokio::task::spawn(async move {
        conversion::convert_files(files, converter, &args.output_extension)
            .await
            .wrap_err("Failed to convert files")
    });

    tokio::pin!(converted_task);
    let r = (&mut converted_task).await?;

    if let Err(e) = r {
        error!("{:?}", e);
        return Err(Error::Generic(format!("Failed to convert files due to: {}", e)));
    }

    info!("Successfully converted all files");

    Ok(())
}
