use std::io::Cursor;
use anyhow::{Context, Result};
use once_cell::sync::OnceCell;

static TINY_PNG: OnceCell<Vec<u8>> = OnceCell::new();

/// Returns 1×1 transparent PNG bytes (computed once).
pub fn tiny_empty_png() -> &'static [u8] {
    TINY_PNG.get_or_init(|| {
        use image::{ImageBuffer, Rgba, ImageFormat};
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 0]));
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .expect("encode tiny png");
        buf
    })
}

/// LoadImageFromFile loads a PNG or JPEG image from a local file path.
pub fn load_image_from_file(path: &str) -> Result<Vec<u8>> {
    std::fs::read(path).with_context(|| format!("open file: {}", path))
}

/// LoadImageFromURL fetches an image from the given URL.
pub fn load_image_from_url(url: &str) -> Result<Vec<u8>> {
    let resp = reqwest::blocking::get(url)
        .with_context(|| format!("fetch URL: {}", url))?;
    if !resp.status().is_success() {
        anyhow::bail!("unexpected status {} fetching {}", resp.status(), url);
    }
    resp.bytes()
        .map(|b| b.to_vec())
        .context("read response bytes")
}
