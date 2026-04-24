use std::io::Read;

/// Anchor represents a reference point on a PDF page for positioning text and images.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,     // 0
    TopCenter,   // 1
    TopRight,    // 2
    Left,        // 3
    Center,      // 4
    Right,       // 5
    BottomLeft,  // 6
    BottomCenter,// 7
    BottomRight, // 8
}

/// TextField defines a text value and its position on the certificate form.
#[derive(Debug, Clone)]
pub struct TextField {
    pub text: String,
    pub dx: f64,
    pub dy: f64,
    pub font_size: i32,
    pub font_name: String,
    pub position: Anchor,
}

impl TextField {
    pub fn new(text: impl Into<String>, dx: f64, dy: f64, font_size: i32, position: Anchor) -> Self {
        Self {
            text: text.into(),
            dx,
            dy,
            font_size,
            font_name: "THSarabunNew".to_string(),
            position,
        }
    }
}

/// ImageField defines an image (signature or seal) and its position on the certificate form.
pub struct ImageField {
    pub data: Vec<u8>,
    pub pos: Anchor,
    pub dx: f64,
    pub dy: f64,
    pub scale: f64,
    pub opacity: f64,
    pub diagonal: i32,
    pub on_top: bool,
}

impl ImageField {
    pub fn from_reader<R: Read>(mut reader: R, pos: Anchor, dx: f64, dy: f64, scale: f64, opacity: f64, diagonal: i32, on_top: bool) -> std::io::Result<Self> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        Ok(Self { data, pos, dx, dy, scale, opacity, diagonal, on_top })
    }
}
