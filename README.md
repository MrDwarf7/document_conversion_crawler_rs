# Document Conversion Crawler (Rust)

A high-performance, asynchronous CLI tool that recursively crawls directories to find and convert document files using embedded Pandoc. Built with Rust for speed, safety, and reliability.

## Features

- **🚀 Blazing Fast**: Asynchronous parallel conversion using Tokio
- **📦 Self-Contained**: Embeds UPX-compressed Pandoc binary (Windows) - no external dependencies required
- **🔍 Smart Crawling**: Recursively searches directories for files by extension
- **🛡️ Robust**: Automatically fixes problematic filenames containing `$` or `~` characters
- **📊 Detailed Logging**: Configurable verbosity levels (Error, Warn, Info, Debug, Trace)
- **🎯 Flexible Output**: Convert in-place or to a custom output directory
- **📁 Media Extraction**: Automatically extracts and organizes embedded media from documents
- **⚡ Efficient**: Uses cranelift backend for faster compilation during development

## Installation

### Prerequisites

- Rust 1.83+ (uses edition 2024)
- Cargo

### Build from Source

```bash
git clone https://github.com/MrDwarf7/document_conversion_crawler_rs.git
cd document_conversion_crawler_rs
cargo build --release
```

The compiled binary will be in `target/release/document_conversion_crawler_rs`

## Usage

### Basic Syntax

```bash
document_conversion_crawler_rs <INPUT_DIR> <INPUT_EXT> <OUTPUT_EXT> [OPTIONS]
```

### Arguments

- `<INPUT_DIR>` - Root directory to crawl for files
- `<INPUT_EXT>` - Input file extension to search for (e.g., `docx`, `.docx`)
- `<OUTPUT_EXT>` - Output format extension (e.g., `md`, `html`, `pdf`)

### Options

- `-o, --output <DIR>` - Custom output directory for converted files
- `-l, --level_verbosity <LEVEL>` - Logging verbosity (ERROR/0, WARN/1, INFO/2, DEBUG/3, TRACE/4)
  - Default: INFO

### Examples

#### Convert all .docx files to .md in the same directory

```bash
document_conversion_crawler_rs ./documents docx md
```

#### Convert with custom output directory

```bash
document_conversion_crawler_rs ./documents docx md -o ./converted
```

#### Convert with debug logging

```bash
document_conversion_crawler_rs ./documents docx html -l DEBUG
```

#### Convert with numeric verbosity level

```bash
document_conversion_crawler_rs ./documents docx pdf -l 3
```

## How It Works

1. **Initialization**: The tool initializes the async runtime and logger
2. **Pandoc Setup**: Extracts the embedded Pandoc binary to the system temp directory (Windows)
3. **Directory Crawling**: Recursively walks the input directory tree
4. **Filename Sanitization**: Fixes problematic filenames containing `$` or `~` characters
5. **File Discovery**: Collects all files matching the input extension
6. **Parallel Conversion**: Spawns async tasks to convert files concurrently
7. **Media Extraction**: Creates `<filename>/media/` folders for extracted document media
8. **Output Organization**: Places converted files in the output directory (if specified)
9. **Progress Reporting**: Logs conversion progress and provides success statistics

## Architecture

### Module Structure

```
src/
├── main.rs              # Application entry point and orchestration
├── prelude.rs           # Common imports, utilities, and pandoc embedding
├── error.rs             # Custom error types using thiserror
├── cli.rs               # Command-line argument parsing with clap
├── lazy_logger.rs       # Buffered logger implementation
└── conversion/
    ├── mod.rs           # Core conversion logic and file discovery
    └── pandoc.rs        # Pandoc converter implementation
```

### Key Components

#### Converter Trait

The `Converter` trait provides an abstraction for different conversion backends:

```rust
#[async_trait::async_trait]
pub trait Converter {
    async fn convert(&self, input: PathBuf, output: PathBuf) -> Result<()>;
    async fn check_installed(&self) -> Result<bool>;
    fn name(&self) -> impl AsRef<str>;
}
```

#### Embedded Pandoc

On Windows, the tool embeds a UPX-compressed Pandoc binary (~30MB → ~10MB) directly into the executable. On first run, it extracts the binary to:

```
<TEMP_DIR>/pandoc_upx.exe
```

This eliminates the need for users to install Pandoc separately.

#### Async Task Spawning

Each file conversion runs in a separate Tokio task, enabling parallel processing:

```rust
let tasks: Vec<_> = files
    .into_iter()
    .map(|file| tokio::task::spawn(async move {
        converter.convert(file, output).await
    }))
    .collect();
```

## Supported Formats

The tool supports any format that Pandoc supports, including:

**Input Formats**: docx, odt, epub, html, latex, markdown, rst, textile, org, and more

**Output Formats**: markdown, html, pdf, docx, epub, latex, rst, org, and more

See [Pandoc's documentation](https://pandoc.org/MANUAL.html) for the complete list.

## Error Handling

The tool provides detailed error messages for common issues:

- **File Access**: Permission denied, file not found
- **Conversion Failures**: Invalid input format, corrupted files
- **Directory Issues**: Cannot create output directories
- **Pandoc Errors**: Stderr output from Pandoc is captured and logged

Example error output:

```
ERROR: Failed to convert files due to: Pandoc conversion error, failed for: document.docx
```

## Performance

- **Concurrent Execution**: Processes multiple files simultaneously
- **Optimized Binary**: Release builds use `opt-level = 3` and single codegen unit
- **Development Speed**: Uses cranelift backend for faster compilation
- **Efficient Dependencies**: Minimal dependency tree focused on performance

## Configuration

### Cargo.toml Features

- **Edition 2024**: Uses the latest Rust edition
- **Cranelift Backend**: Fast compilation in development mode
- **Optimized Dependencies**: All dependencies compiled with `opt-level = 3`

### Environment

The tool respects standard Rust environment variables:

- `RUST_LOG`: Override logging levels (e.g., `RUST_LOG=debug`)
- `RUST_BACKTRACE`: Enable backtraces on panic

## Logging

The tool uses `tracing` and `tracing-subscriber` for structured logging:

- Line numbers and thread IDs included
- ANSI color support
- Configurable log levels per module
- Timestamp information available

Example output:

```
INFO: Found 42 files to convert
INFO: Running conversion for 42 files
DEBUG: Converting 'report.docx' to 'report.md'
INFO: Successfully converted all files
INFO: Processed a total of: 42 files
INFO: Successfully processed: 42 files
INFO: Success rate: 100.00%
```

## Limitations

- Embedded Pandoc binary is Windows-only (Linux/Mac users need Pandoc installed separately)
- Zipped output from conversion tasks may mismatch if top-level folders < individual files
- File overwrites are skipped (warns if output exists)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development Setup

```bash
# Clone the repository
git clone https://github.com/MrDwarf7/document_conversion_crawler_rs.git
cd document_conversion_crawler_rs

# Run with watch mode (requires cargo-watch)
cargo watch -q -c -w src/ -x run

# Run tests
cargo test

# Run with development optimizations
cargo build
```

## License

Due to the direct inclusion of the pandoc binary, this project requires
the code be licensed under "GNU General Public License v2.0".
All conditions of the currently provided license apply
based on the requirements specified under the [pandoc project](https://github.com/jgm/pandoc).

## Dependencies

- **tokio**: Async runtime
- **clap**: CLI argument parsing
- **tracing**: Structured logging
- **walkdir**: Directory traversal
- **eyre**: Error handling
- **thiserror**: Custom error types
- **async-trait**: Async trait support

## Acknowledgments

- Uses [Pandoc](https://pandoc.org/) for document conversion
- Compressed with [UPX](https://upx.github.io/) for smaller binary size

## Author

### Blake B. // MrDwarf7

## Support

For issues, feature requests, or questions, please open an issue on GitHub.

---
