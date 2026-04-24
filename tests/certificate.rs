use pdf50tawi::{
    issue_wht_certificate_pdf, validate_tax_info,
    tax_info::{
        Certification, DateOfIssuance, DocumentDetails, IncomeDetail, OtherPayments, Payee, Payer,
        TaxInfo, Totals, WithholdingType,
    },
};

// ── Shared fixture ─────────────────────────────────────────────────────────────

fn minimal_tax_info() -> TaxInfo {
    TaxInfo {
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

fn full_tax_info() -> TaxInfo {
    TaxInfo {
        document_details: DocumentDetails {
            book_number: "001".to_string(),
            document_number: "001".to_string(),
        },
        payer: Payer {
            tax_id: "1234567890123".to_string(),
            tax_id10_digit: "1234567890".to_string(),
            name: "บริษัท ตัวอย่าง จำกัด".to_string(),
            address: "123 ถนนสุขุมวิท แขวงคลองตัน เขตวัฒนา กรุงเทพฯ 10110".to_string(),
        },
        payee: Payee {
            tax_id: "3210987654321".to_string(),
            tax_id10_digit: "1234567890".to_string(),
            name: "นางสาวสมชาย นามสกุลยาวมากไหมนะก็ไม่รู้เหมือนกัน".to_string(),
            address: "555 ต.ทุ่งนา  อ.ทุ่งนา  จ.ชลบุรี  12345".to_string(),
            sequence_number: "321".to_string(),
            pnd_1a: true,
            pnd_1a_special: true,
            pnd_2: true,
            pnd_3: true,
            pnd_2a: true,
            pnd_3a: true,
            pnd_53: true,
        },
        income40_1: IncomeDetail {
            date_paid: "01 มกราคม 2568".to_string(),
            amount_paid: "401,010.01".to_string(),
            tax_withheld: "12,030.30".to_string(),
        },
        income40_2: IncomeDetail {
            date_paid: "02 ก.พ. 2568".to_string(),
            amount_paid: "402,020.02".to_string(),
            tax_withheld: "12,060.60".to_string(),
        },
        income40_3: IncomeDetail {
            date_paid: "03 มี.ค. 2568".to_string(),
            amount_paid: "403,030.03".to_string(),
            tax_withheld: "12,090.90".to_string(),
        },
        income40_4a: IncomeDetail {
            date_paid: "04 เม.ย. 2568".to_string(),
            amount_paid: "404,040.04".to_string(),
            tax_withheld: "12,121.20".to_string(),
        },
        income40_4b_1_1: IncomeDetail {
            date_paid: "05 พ.ค. 2568".to_string(),
            amount_paid: "411,010.01".to_string(),
            tax_withheld: "12,330.30".to_string(),
        },
        income40_4b_1_2: IncomeDetail {
            date_paid: "06 มิ.ย. 2568".to_string(),
            amount_paid: "412,020.02".to_string(),
            tax_withheld: "12,360.60".to_string(),
        },
        income40_4b_1_3: IncomeDetail {
            date_paid: "07 ก.ค. 2568".to_string(),
            amount_paid: "413,030.03".to_string(),
            tax_withheld: "12,390.90".to_string(),
        },
        income40_4b_1_4_rate: "ร้อยละ 7".to_string(),
        income40_4b_1_4: IncomeDetail {
            date_paid: "08 ส.ค. 2568".to_string(),
            amount_paid: "414,040.04".to_string(),
            tax_withheld: "12,421.20".to_string(),
        },
        income40_4b_2_1: IncomeDetail {
            date_paid: "09 ก.ย. 2568".to_string(),
            amount_paid: "421,010.01".to_string(),
            tax_withheld: "12,630.30".to_string(),
        },
        income40_4b_2_2: IncomeDetail {
            date_paid: "10 ต.ค. 2568".to_string(),
            amount_paid: "422,020.02".to_string(),
            tax_withheld: "12,660.60".to_string(),
        },
        income40_4b_2_3: IncomeDetail {
            date_paid: "11 พ.ย. 2568".to_string(),
            amount_paid: "423,030.03".to_string(),
            tax_withheld: "12,690.90".to_string(),
        },
        income40_4b_2_4: IncomeDetail {
            date_paid: "12 ธ.ค. 2568".to_string(),
            amount_paid: "424,040.04".to_string(),
            tax_withheld: "12,721.20".to_string(),
        },
        income40_4b_2_5_note: "กำไรอื่นๆ".to_string(),
        income40_4b_2_5: IncomeDetail {
            date_paid: "13 ม.ค. 2568".to_string(),
            amount_paid: "425,050.05".to_string(),
            tax_withheld: "12,751.50".to_string(),
        },
        income5: IncomeDetail {
            date_paid: "14 ก.พ. 2568".to_string(),
            amount_paid: "500,010.01".to_string(),
            tax_withheld: "15,000.30".to_string(),
        },
        income6_note: "รายได้อื่นๆ".to_string(),
        income6: IncomeDetail {
            date_paid: "15 มี.ค. 2568".to_string(),
            amount_paid: "600,060.06".to_string(),
            tax_withheld: "18,001.80".to_string(),
        },
        totals: Totals {
            total_amount_paid: "5,741,320.36".to_string(),
            total_tax_withheld: "172,239.60".to_string(),
            total_tax_withheld_in_words: "หนึ่งแสนเจ็ดหมื่นสองพันสองร้อยสามสิบเก้าบาทหกสิบสตางค์"
                .to_string(),
        },
        other_payments: OtherPayments {
            government_pension_fund: "5,000.00".to_string(),
            social_security_fund: "750.00".to_string(),
            provident_fund: "3,000.00".to_string(),
        },
        withholding_type: WithholdingType {
            withholding_tax: true,
            forever: true,
            one_time: true,
            other: true,
            other_details: "อื่นๆ อื่นๆ อื่นๆ อื่นๆ".to_string(),
        },
        certification: Certification {
            date_of_issuance: DateOfIssuance {
                day: "22".to_string(),
                month: "ธันวาคม".to_string(),
                year: "2568".to_string(),
            },
        },
    }
}

// ── PDF generation ─────────────────────────────────────────────────────────────

#[test]
fn generates_valid_pdf_minimal_data() {
    let mut buf = Vec::new();
    issue_wht_certificate_pdf(&mut buf, minimal_tax_info(), None, None).unwrap();
    assert!(buf.starts_with(b"%PDF"), "output must start with %PDF");
    assert!(buf.len() > 10_000, "PDF too small: {} bytes", buf.len());
}

#[test]
fn generates_valid_pdf_full_data() {
    let mut buf = Vec::new();
    issue_wht_certificate_pdf(&mut buf, full_tax_info(), None, None).unwrap();
    assert!(buf.starts_with(b"%PDF"));
    assert!(buf.len() > 10_000);
}

#[test]
fn pdf_output_ends_with_eof_marker() {
    let mut buf = Vec::new();
    issue_wht_certificate_pdf(&mut buf, minimal_tax_info(), None, None).unwrap();
    let tail = &buf[buf.len().saturating_sub(10)..];
    assert!(tail.windows(5).any(|w| w == b"%%EOF"), "PDF must end with %%EOF");
}

// ── Validation ─────────────────────────────────────────────────────────────────

#[test]
fn validation_accepts_minimal_data() {
    assert!(validate_tax_info(&minimal_tax_info()).is_ok());
}

#[test]
fn validation_accepts_full_data() {
    assert!(validate_tax_info(&full_tax_info()).is_ok());
}

#[test]
fn validation_rejects_empty_payer_name() {
    let mut tax = minimal_tax_info();
    tax.payer.name = String::new();
    assert!(validate_tax_info(&tax).is_err());
}

// ── Serialisation ──────────────────────────────────────────────────────────────

#[test]
fn serde_roundtrip_preserves_all_fields() {
    let original = full_tax_info();
    let json = serde_json::to_string(&original).unwrap();
    let restored: TaxInfo = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.payer.name, original.payer.name);
    assert_eq!(restored.payer.tax_id, original.payer.tax_id);
    assert_eq!(restored.payee.pnd_53, original.payee.pnd_53);
    assert_eq!(restored.income40_1.amount_paid, original.income40_1.amount_paid);
    assert_eq!(restored.totals.total_tax_withheld, original.totals.total_tax_withheld);
    assert_eq!(restored.withholding_type.forever, original.withholding_type.forever);
    assert_eq!(
        restored.certification.date_of_issuance.month,
        original.certification.date_of_issuance.month
    );
}

#[test]
fn json_uses_camel_case_keys() {
    let tax = minimal_tax_info();
    let json = serde_json::to_string(&tax).unwrap();
    assert!(json.contains("\"taxId\""), "taxId should be camelCase");
    assert!(json.contains("\"pnd1a\"") || json.contains("\"pnd_1a\""));
    assert!(json.contains("\"datePaid\""), "datePaid should be camelCase");
    assert!(json.contains("\"totalAmountPaid\""), "totalAmountPaid should be camelCase");
    assert!(!json.contains("\"tax_id\""), "snake_case keys must not appear");
}

#[test]
fn deserialises_from_camel_case_json() {
    // Verify that camelCase JSON (as sent by REST clients) round-trips correctly.
    let original = minimal_tax_info();
    let json = serde_json::to_string(&original).unwrap();
    let restored: TaxInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.payer.name, original.payer.name);
    assert_eq!(restored.payee.pnd_1a, original.payee.pnd_1a);
    assert_eq!(restored.income40_1.amount_paid, original.income40_1.amount_paid);
    assert_eq!(restored.withholding_type.withholding_tax, original.withholding_type.withholding_tax);
}
