use std::io;
use anyhow::Result;

use crate::tax_info::TaxInfo;
use crate::field::{TextField, ImageField, Anchor};
use crate::image_util::tiny_empty_png;
use crate::pdf::fill_certificate;

/// IssueWHTCertificatePDF generates a filled WHT certificate PDF.
pub fn issue_wht_certificate_pdf<W: io::Write>(
    out: &mut W,
    tax_info: TaxInfo,
    sign: Option<Vec<u8>>,
    logo: Option<Vec<u8>>,
) -> Result<()> {
    let images = certificate_image_fields(sign, logo)?;
    let texts = text_fields_from_tax_info(&tax_info);
    fill_certificate(texts, images, out)
}

/// CertificateImageFields returns the positioned image fields for the signature and company seal.
pub fn certificate_image_fields(sign: Option<Vec<u8>>, logo: Option<Vec<u8>>) -> Result<Vec<ImageField>> {
    let sign_data = sign.unwrap_or_else(|| tiny_empty_png().to_vec());
    let logo_data = logo.unwrap_or_else(|| tiny_empty_png().to_vec());

    Ok(vec![
        ImageField {
            data: sign_data,
            pos: Anchor::Center,
            dx: 86.0,
            dy: -313.0,
            scale: 0.1,
            opacity: 1.0,
            diagonal: 0,
            on_top: true,
        },
        ImageField {
            data: logo_data,
            pos: Anchor::Center,
            dx: 212.0,
            dy: -325.0,
            scale: 0.06,
            opacity: 1.0,
            diagonal: 1,
            on_top: false,
        },
    ])
}

/// TextFieldsFromTaxInfo converts TaxInfo into the complete set of TextField values
/// to be rendered on the certificate form.
pub fn text_fields_from_tax_info(tax: &TaxInfo) -> Vec<TextField> {
    use Anchor::*;

    // Payer Information (ผู้จ่ายเงิน)
    let mut payer = vec![
        TextField::new(&tax.payer.name, 58.0, -110.0, 14, TopLeft),
        TextField::new(&tax.payer.address, 62.0, -132.0, 12, TopLeft),
    ];
    payer.extend(position_tax_id13_digits(&tax.payer.tax_id, -94.0, 16));
    payer.extend(position_tax_id10_digits(&tax.payer.tax_id10_digit, -111.0, 16));

    // Payee Information (ผู้ถูกหักภาษี ณ ที่จ่าย)
    let mut payee = vec![
        TextField::new(&tax.payee.name, 58.0, -182.0, 14, TopLeft),
        TextField::new(&tax.payee.address, 62.0, -208.0, 12, TopLeft),
    ];
    payee.extend(position_tax_id13_digits(&tax.payee.tax_id, -163.0, 16));
    payee.extend(position_tax_id10_digits(&tax.payee.tax_id10_digit, -182.0, 16));
    payee.extend(vec![
        // Tax Filing Reference (ลำดับที่)
        TextField::new(&tax.payee.sequence_number, -190.0, -236.0, 14, TopCenter),

        checkmark(tax.payee.pnd_1a, 211.5, -230.0),
        checkmark(tax.payee.pnd_1a_special, 289.0, -230.0),
        checkmark(tax.payee.pnd_2, 397.0, -230.0),
        checkmark(tax.payee.pnd_2a, 211.5, -248.0),
        checkmark(tax.payee.pnd_3, 474.0, -230.0),
        checkmark(tax.payee.pnd_3a, 289.0, -248.0),
        checkmark(tax.payee.pnd_53, 397.0, -248.0),
    ]);

    let mut text_fields: Vec<TextField> = Vec::with_capacity(128);
    text_fields.extend(vec![
        // Document Details (top right)
        TextField::new(&tax.document_details.book_number, 519.0, -59.0, 14, TopLeft),
        TextField::new(&tax.document_details.document_number, 519.0, -74.0, 14, TopLeft),

        // Income Details - Row 1 (เงินเดือน ค่าจาง)
        TextField::new(&tax.income40_1.date_paid, 69.0, 536.0, 14, BottomCenter),
        TextField::new(&tax.income40_1.amount_paid, -109.5, 536.0, 14, BottomRight),
        TextField::new(&tax.income40_1.tax_withheld, -38.0, 536.0, 14, BottomRight),

        // Income Details - Row 2 (ค่าธรรมเนียม ค่านายหน้า)
        TextField::new(&tax.income40_2.date_paid, 69.0, 522.0, 14, BottomCenter),
        TextField::new(&tax.income40_2.amount_paid, -109.5, 522.0, 14, BottomRight),
        TextField::new(&tax.income40_2.tax_withheld, -38.0, 522.0, 14, BottomRight),

        // Income Details - Row 3 (ค่าแห่งลิขสิทธิ์)
        TextField::new(&tax.income40_3.date_paid, 69.0, 508.0, 14, BottomCenter),
        TextField::new(&tax.income40_3.amount_paid, -109.5, 508.0, 14, BottomRight),
        TextField::new(&tax.income40_3.tax_withheld, -38.0, 508.0, 14, BottomRight),

        // Income Details - Row 4 (ดอกเบี้ย เงินปันผล) 40 (4) (ก)
        TextField::new(&tax.income40_4a.date_paid, 69.0, 494.0, 14, BottomCenter),
        TextField::new(&tax.income40_4a.amount_paid, -109.5, 494.0, 14, BottomRight),
        TextField::new(&tax.income40_4a.tax_withheld, -38.0, 494.0, 14, BottomRight),

        // Income Details 40 (4) (ข) (1) (1.1)
        TextField::new(&tax.income40_4b_1_1.date_paid, 69.0, 437.0, 14, BottomCenter),
        TextField::new(&tax.income40_4b_1_1.amount_paid, -109.5, 437.0, 14, BottomRight),
        TextField::new(&tax.income40_4b_1_1.tax_withheld, -38.0, 437.0, 14, BottomRight),

        // Income Details 40 (4) (ข) (1) (1.2)
        TextField::new(&tax.income40_4b_1_2.date_paid, 69.0, 420.0, 14, BottomCenter),
        TextField::new(&tax.income40_4b_1_2.amount_paid, -109.5, 420.0, 14, BottomRight),
        TextField::new(&tax.income40_4b_1_2.tax_withheld, -38.0, 420.0, 14, BottomRight),

        // Income Details 40 (4) (ข) (1) (1.3)
        TextField::new(&tax.income40_4b_1_3.date_paid, 69.0, 406.0, 14, BottomCenter),
        TextField::new(&tax.income40_4b_1_3.amount_paid, -109.5, 406.0, 14, BottomRight),
        TextField::new(&tax.income40_4b_1_3.tax_withheld, -38.0, 406.0, 14, BottomRight),

        // Income Details 40 (4) (ข) (1) (1.4)
        TextField::new(&tax.income40_4b_1_4_rate, -116.0, 390.0, 14, BottomCenter),
        TextField::new(&tax.income40_4b_1_4.date_paid, 69.0, 391.0, 14, BottomCenter),
        TextField::new(&tax.income40_4b_1_4.amount_paid, -109.5, 391.0, 14, BottomRight),
        TextField::new(&tax.income40_4b_1_4.tax_withheld, -38.0, 391.0, 14, BottomRight),

        // Income Details 40 (4) (ข) (2) (2.1)
        TextField::new(&tax.income40_4b_2_1.date_paid, 69.0, 362.0, 14, BottomCenter),
        TextField::new(&tax.income40_4b_2_1.amount_paid, -109.5, 362.0, 14, BottomRight),
        TextField::new(&tax.income40_4b_2_1.tax_withheld, -38.0, 362.0, 14, BottomRight),

        // Income Details 40 (4) (ข) (2) (2.2)
        TextField::new(&tax.income40_4b_2_2.date_paid, 69.0, 333.0, 14, BottomCenter),
        TextField::new(&tax.income40_4b_2_2.amount_paid, -109.5, 333.0, 14, BottomRight),
        TextField::new(&tax.income40_4b_2_2.tax_withheld, -38.0, 333.0, 14, BottomRight),

        // Income Details 40 (4) (ข) (2) (2.3)
        TextField::new(&tax.income40_4b_2_3.date_paid, 69.0, 304.0, 14, BottomCenter),
        TextField::new(&tax.income40_4b_2_3.amount_paid, -109.5, 304.0, 14, BottomRight),
        TextField::new(&tax.income40_4b_2_3.tax_withheld, -38.0, 304.0, 14, BottomRight),

        // Income Details 40 (4) (ข) (2) (2.4)
        TextField::new(&tax.income40_4b_2_4.date_paid, 69.0, 288.0, 14, BottomCenter),
        TextField::new(&tax.income40_4b_2_4.amount_paid, -109.5, 288.0, 14, BottomRight),
        TextField::new(&tax.income40_4b_2_4.tax_withheld, -38.0, 288.0, 14, BottomRight),

        // Income Details 40 (4) (ข) (2) (2.5)
        TextField::new(&tax.income40_4b_2_5_note, 150.0, 275.0, 12, BottomLeft),
        TextField::new(&tax.income40_4b_2_5.date_paid, 69.0, 275.0, 14, BottomCenter),
        TextField::new(&tax.income40_4b_2_5.amount_paid, -109.5, 275.0, 14, BottomRight),
        TextField::new(&tax.income40_4b_2_5.tax_withheld, -38.0, 275.0, 14, BottomRight),

        // Income Details Row 7 — 5. การจ่ายเงินได้ที่ต้องหักภาษี ณ ที่จ่าย
        TextField::new(&tax.income5.date_paid, 69.0, 217.0, 14, BottomCenter),
        TextField::new(&tax.income5.amount_paid, -109.5, 217.0, 14, BottomRight),
        TextField::new(&tax.income5.tax_withheld, -38.0, 217.0, 14, BottomRight),

        // Income Details Row 8 — 6. อื่น ๆ (ระบุ)
        TextField::new(&tax.income6_note, 102.0, 203.0, 12, BottomLeft),
        TextField::new(&tax.income6.date_paid, 69.0, 203.0, 14, BottomCenter),
        TextField::new(&tax.income6.amount_paid, -109.5, 203.0, 14, BottomRight),
        TextField::new(&tax.income6.tax_withheld, -38.0, 203.0, 14, BottomRight),

        // Totals (รวม)
        TextField::new(&tax.totals.total_amount_paid, -109.5, 182.0, 14, BottomRight),
        TextField::new(&tax.totals.total_tax_withheld, -38.0, 182.0, 14, BottomRight),
        TextField::new(&tax.totals.total_tax_withheld_in_words, 200.0, 163.0, 14, BottomLeft),

        // Other Payments (เงินที่จ่ายเข้ากองทุน)
        TextField::new(&tax.other_payments.government_pension_fund, -318.0, 146.0, 12, BottomRight),
        TextField::new(&tax.other_payments.social_security_fund, -190.0, 146.0, 12, BottomRight),
        TextField::new(&tax.other_payments.provident_fund, -54.0, 146.0, 12, BottomRight),

        // Withholding Type
        checkmark(tax.withholding_type.withholding_tax, 85.0, -712.0),
        checkmark(tax.withholding_type.forever, 178.0, -712.0),
        checkmark(tax.withholding_type.one_time, 285.5, -712.0),
        checkmark(tax.withholding_type.other, 396.0, -712.0),
        TextField::new(&tax.withholding_type.other_details, 470.0, 124.0, 12, BottomLeft),

        // Certification (ลงชื่อ ผู้จ่ายเงิน และวันที่)
        TextField::new(&tax.certification.date_of_issuance.day, 52.0, 77.0, 14, BottomCenter),
        TextField::new(&tax.certification.date_of_issuance.month, 99.0, 77.0, 14, BottomCenter),
        TextField::new(&tax.certification.date_of_issuance.year, 152.0, 77.0, 14, BottomCenter),
    ]);

    text_fields.extend(payer);
    text_fields.extend(payee);

    filter_empty_text_fields(text_fields)
}

/// positionTaxID13Digits creates individual text fields for each digit of a 13-digit tax ID.
pub fn position_tax_id13_digits(tax_id: &str, dy: f64, font_size: i32) -> Vec<TextField> {
    let digits: String = tax_id.replace(' ', "");
    let x_positions = [378.0f64, 396.0, 408.0, 420.0, 432.0, 450.0, 463.0, 474.0, 486.0, 498.0, 517.0, 529.0, 548.0];
    position_digits(&digits, font_size, dy, &x_positions)
}

/// positionTaxID10Digits creates individual text fields for each digit of a 10-digit tax ID.
pub fn position_tax_id10_digits(tax_id: &str, dy: f64, font_size: i32) -> Vec<TextField> {
    let digits: String = tax_id.replace(' ', "");
    let x_positions = [422.0f64, 440.0, 452.0, 464.0, 476.0, 494.0, 506.0, 518.0, 530.0, 548.0];
    position_digits(&digits, font_size, dy, &x_positions)
}

fn position_digits(digits: &str, font_size: i32, dy: f64, x_positions: &[f64]) -> Vec<TextField> {
    digits.chars()
        .enumerate()
        .filter_map(|(i, ch)| {
            x_positions.get(i).map(|&dx| {
                TextField::new(ch.to_string(), dx, dy, font_size, Anchor::TopLeft)
            })
        })
        .collect()
}

fn tick(is_set: bool) -> &'static str {
    if is_set { "✓" } else { "" }
}

fn checkmark(is_set: bool, dx: f64, dy: f64) -> TextField {
    TextField::new(tick(is_set), dx, dy, 10, Anchor::TopLeft)
}

fn filter_empty_text_fields(fields: Vec<TextField>) -> Vec<TextField> {
    fields.into_iter()
        .filter(|f| !f.text.trim().is_empty())
        .collect()
}
