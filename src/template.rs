static TEMPLATE_BYTES: &[u8] = include_bytes!("../assets/form/tax50tawiTemplate.pdf");

/// Returns the raw template PDF bytes.
pub(crate) fn template_bytes() -> &'static [u8] {
    TEMPLATE_BYTES
}
