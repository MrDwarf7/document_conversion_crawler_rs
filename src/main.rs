mod cli;
mod conversion;
mod error;
mod lazy_logger;
mod prelude;

use std::sync::Arc;

use conversion::{Converter, ProcessableEntities};
use eyre::WrapErr;

pub use self::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Cli::new();
    let level = args.verbosity_level().into();
    let _ = init_logger(level).init();

    let converter = Arc::new(conversion::pandoc::PandocConverter::new());

    let processable = conversion::find_by_ext(&args.input_directory, &args.input_extension).await?;
    info!("Found {} files to convert", processable.individual_files.len());

    // perhaps we use channels to send/recv. The Command output into a bytes channel buffer
    // -- AIM: smooth out the programm calls to pandoc binary (beofre using a native rs lib)

    let res = convert_call(args, processable, converter).await;

    if let Err(e) = res {
        error!("{:?}", e);
        return Err(Error::Generic(format!("Failed to convert files due to: {}", e)));
    }

    info!("Successfully converted all files");

    Ok(())
}

async fn convert_call<C: Converter + Send + Sync + 'static>(
    args: cli::Cli,
    processable: ProcessableEntities,
    converter: Arc<C>,
) -> Result<()> {
    let converted_tasks = if let Some(output_dir) = args.output_directory {
        if !output_dir.exists() {
            std::fs::create_dir_all(&output_dir).map_err(|e| {
                Error::Generic(format!("Failed to create output directory: {:?} due to: {}", output_dir, e))
            })?;
        }

        tokio::task::spawn(async move {
            conversion::convert_files_with_output(processable, converter, &args.output_extension, &output_dir)
                .await
                .wrap_err("Failed to convert files")
                .map_err(|e| Error::Generic(format!("Failed to convert files due to: {}", e)))
        })
    } else {
        tokio::task::spawn(async move {
            conversion::convert_files(processable, converter, &args.output_extension)
                .await
                .wrap_err("Failed to convert files")
                .map_err(|e| Error::Generic(format!("Failed to convert files due to: {}", e)))
        })
    };

    tokio::pin!(converted_tasks);

    (&mut converted_tasks).await?
}
