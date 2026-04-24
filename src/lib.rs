pub mod tax_info;
pub mod field;
pub mod validate;
pub mod certificate;
pub mod pdf;
pub mod template;
pub mod image_util;
pub mod font;

pub use tax_info::*;
pub use field::*;
pub use validate::*;
pub use certificate::*;
pub use image_util::{load_image_from_file, load_image_from_url};

#[cfg(test)]
mod integration_tests {
    use super::*;

    fn minimal_tax_info() -> TaxInfo {
        TaxInfo {
            document_details: DocumentDetails {
                book_number: "1".to_string(),
                document_number: "2568001".to_string(),
            },
            payer: Payer {
                name: "บริษัท ตัวอย่าง จำกัด".to_string(),
                tax_id: "1234567890123".to_string(),
                ..Default::default()
            },
            payee: Payee {
                name: "นาย ทดสอบ ระบบ".to_string(),
                tax_id: "9876543210987".to_string(),
                pnd_1a: true,
                ..Default::default()
            },
            income40_1: IncomeDetail {
                date_paid: "15/01/2568".to_string(),
                amount_paid: "50,000.00".to_string(),
                tax_withheld: "2,500.00".to_string(),
            },
            withholding_type: WithholdingType {
                withholding_tax: true,
                ..Default::default()
            },
            totals: Totals {
                total_amount_paid: "50,000.00".to_string(),
                total_tax_withheld: "2,500.00".to_string(),
                total_tax_withheld_in_words: "สองพันห้าร้อยบาทถ้วน".to_string(),
            },
            certification: Certification {
                date_of_issuance: DateOfIssuance {
                    day: "31".to_string(),
                    month: "01".to_string(),
                    year: "2568".to_string(),
                },
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_generate_pdf_no_images() {
        let tax_info = minimal_tax_info();
        assert!(validate_tax_info(&tax_info).is_ok());

        let mut buf = Vec::new();
        let result = issue_wht_certificate_pdf(&mut buf, tax_info, None, None);
        assert!(result.is_ok(), "PDF generation failed: {:?}", result.err());
        // PDF must start with %PDF
        assert!(buf.starts_with(b"%PDF"), "Output doesn't start with %PDF");
        assert!(buf.len() > 1000, "PDF output too small: {} bytes", buf.len());
    }

    #[test]
    fn test_text_fields_from_tax_info() {
        let tax_info = minimal_tax_info();
        let fields = text_fields_from_tax_info(&tax_info);
        assert!(!fields.is_empty(), "Should have text fields");
        // Check payer name is in fields
        let has_name = fields.iter().any(|f| f.text.contains("ตัวอย่าง"));
        assert!(has_name, "Payer name should be in text fields");
    }

    #[test]
    fn test_tax_id_positioning() {
        let fields = position_tax_id13_digits("1234567890123", -94.0, 16);
        assert_eq!(fields.len(), 13);
        assert_eq!(fields[0].text, "1");
        assert_eq!(fields[0].dx, 378.0);
        assert_eq!(fields[12].text, "3");
        assert_eq!(fields[12].dx, 548.0);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let tax_info = minimal_tax_info();
        let json = serde_json::to_string(&tax_info).unwrap();
        let back: TaxInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.payer.name, tax_info.payer.name);
        assert_eq!(back.payee.pnd_1a, tax_info.payee.pnd_1a);
    }
}
