use once_cell::sync::OnceCell;
use tempfile::NamedTempFile;
use anyhow::{Context, Result};

/// Embedded template PDF bytes
static TEMPLATE_BYTES: &[u8] = include_bytes!(
    "../assets/form/tax50tawiTemplate.pdf"
);

/// Returns the raw template PDF bytes.
pub fn template_bytes() -> &'static [u8] {
    TEMPLATE_BYTES
}

static TEMPLATE_PATH: OnceCell<String> = OnceCell::new();

/// Returns a path to a temp file containing the template PDF.
/// The file is written once and reused for the process lifetime.
pub fn cached_template_path() -> Result<&'static str> {
    let path = TEMPLATE_PATH.get_or_try_init(|| -> Result<String> {
        let mut f = NamedTempFile::new().context("create temp file")?;
        use std::io::Write;
        f.write_all(TEMPLATE_BYTES).context("write template bytes")?;
        // Keep the file alive by persisting it (won't be deleted)
        let path = f.into_temp_path();
        let p = path.to_str().unwrap_or("").to_string();
        // Persist to avoid deletion
        path.keep().context("keep temp file")?;
        Ok(p)
    })?;
    Ok(path.as_str())
}
