use std::path::Path;

use tokio::io::AsyncWriteExt;

#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct LazyLogger {
    buf: Vec<u8>,
}

#[allow(dead_code)]
impl LazyLogger {
    pub fn log_input_output(&mut self, input: &Path, output: &Path) {
        self.new_line();
        self.insert(format!("Converting '{}' to '{}'", input.display(), output.display()));
    }

    pub fn insert(&mut self, s: impl AsRef<str>) {
        self.buf.extend_from_slice(s.as_ref().as_bytes());
    }

    #[inline]
    pub fn new_line(&mut self) {
        self.buf.push(b'\n');
    }

    #[inline]
    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub async fn flush_async(&mut self) -> crate::Result<()> {
        self.new_line();
        tokio::io::stdout().write_all(&self.buf).await?;
        self.buf.clear();
        Ok(())
    }
}
