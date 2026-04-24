use std::io::Read;

/// Nine-point anchor system for positioning text and images on a PDF page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// A text value and its position on the certificate form.
#[derive(Debug, Clone)]
pub struct TextField {
    pub text: String,
    pub dx: f64,
    pub dy: f64,
    pub font_size: i32,
    pub position: Anchor,
}

impl TextField {
    pub fn new(text: impl Into<String>, dx: f64, dy: f64, font_size: i32, position: Anchor) -> Self {
        Self {
            text: text.into(),
            dx,
            dy,
            font_size,
            position,
        }
    }
}

/// An image (signature or seal) and its position on the certificate form.
pub struct ImageField {
    pub data: Vec<u8>,
    pub pos: Anchor,
    pub dx: f64,
    pub dy: f64,
    pub scale: f64,
    pub opacity: f64,
    pub on_top: bool,
}

impl ImageField {
    pub fn from_reader<R: Read>(
        mut reader: R,
        pos: Anchor,
        dx: f64,
        dy: f64,
        scale: f64,
        opacity: f64,
        on_top: bool,
    ) -> std::io::Result<Self> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        Ok(Self { data, pos, dx, dy, scale, opacity, on_top })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn text_field_new_stores_all_fields() {
        let f = TextField::new("hello", 10.0, 20.0, 14, Anchor::TopLeft);
        assert_eq!(f.text, "hello");
        assert_eq!(f.dx, 10.0);
        assert_eq!(f.dy, 20.0);
        assert_eq!(f.font_size, 14);
        assert_eq!(f.position, Anchor::TopLeft);
    }

    #[test]
    fn text_field_new_accepts_owned_string() {
        let owned = String::from("owned string");
        let f = TextField::new(owned, 0.0, 0.0, 12, Anchor::Center);
        assert_eq!(f.text, "owned string");
    }

    #[test]
    fn image_field_from_reader_reads_all_bytes() {
        let data = vec![1u8, 2, 3, 4, 5];
        let f = ImageField::from_reader(Cursor::new(data.clone()), Anchor::Center, 5.0, 10.0, 0.5, 1.0, true).unwrap();
        assert_eq!(f.data, data);
        assert_eq!(f.pos, Anchor::Center);
        assert_eq!(f.dx, 5.0);
        assert_eq!(f.dy, 10.0);
        assert!((f.scale - 0.5).abs() < f64::EPSILON);
        assert!(f.on_top);
    }

    #[test]
    fn all_nine_anchor_variants_are_distinct() {
        let variants = [
            Anchor::TopLeft, Anchor::TopCenter, Anchor::TopRight,
            Anchor::Left, Anchor::Center, Anchor::Right,
            Anchor::BottomLeft, Anchor::BottomCenter, Anchor::BottomRight,
        ];
        // Each variant is equal only to itself
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                assert_eq!(a == b, i == j, "{:?} vs {:?}", a, b);
            }
        }
    }
}
