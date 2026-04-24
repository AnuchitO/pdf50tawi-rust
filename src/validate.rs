use crate::tax_info::{TaxInfo, Payee, WithholdingType};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{}", errors.join("; "))]
pub struct ValidationError {
    pub errors: Vec<String>,
}

impl ValidationError {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn add(&mut self, msg: impl Into<String>) {
        self.errors.push(msg.into());
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// ValidateTaxInfo validates all fields in TaxInfo and returns a comprehensive error if any.
pub fn validate_tax_info(t: &TaxInfo) -> Result<(), ValidationError> {
    let mut ve = ValidationError::new();

    validate_party(&mut ve, "payer", &t.payer.name, &t.payer.tax_id, &t.payer.tax_id10_digit);
    validate_party(&mut ve, "payee", &t.payee.name, &t.payee.tax_id, &t.payee.tax_id10_digit);
    validate_payee_pnd(&mut ve, &t.payee);
    validate_withholding_type(&mut ve, &t.withholding_type);

    if ve.has_errors() {
        Err(ve)
    } else {
        Ok(())
    }
}

fn validate_payee_pnd(ve: &mut ValidationError, p: &Payee) {
    if !p.pnd_1a && !p.pnd_1a_special && !p.pnd_2 && !p.pnd_3 && !p.pnd_2a && !p.pnd_3a && !p.pnd_53 {
        ve.add("ผู้ถูกหักภาษี: ต้องเลือกประเภทเงินได้อย่างน้อยหนึ่งประเภท ภ.ง.ด. 1ก: pnd_1a, ภ.ง.ด. 1ก พิเศษ: pnd_1aSpecial, ภ.ง.ด. 2: pnd_2, ภ.ง.ด. 3: pnd_3, ภ.ง.ด. 2ก: pnd_2a, ภ.ง.ด. 3ก: pnd_3a หรือ ภ.ง.ด. 53: pnd_53");
    }
}

fn validate_withholding_type(ve: &mut ValidationError, w: &WithholdingType) {
    if !w.withholding_tax && !w.forever && !w.one_time && !w.other {
        ve.add("ต้องเลือกประเภทหนังสือรับรองอย่างน้อยหนึ่งประเภท (หัก ณ ที่จ่าย: withholdingTax, ออกให้ตลอดไป: forever, ออกให้ครั้งเดียว: oneTime หรือ อื่น ๆ: other)");
    }
}

fn validate_party(ve: &mut ValidationError, prefix: &str, name: &str, tax13: &str, tax10: &str) {
    if name.trim().is_empty() {
        ve.add(format!("{}.name is required", prefix));
    }
    let stripped13 = tax13.replace(' ', "");
    let stripped10 = tax10.replace(' ', "");
    if !stripped13.is_empty() && !is_digits_len(&stripped13, 13) {
        ve.add(format!("{}.taxId must be 13 digits", prefix));
    }
    if !stripped10.is_empty() && !is_digits_len(&stripped10, 10) {
        ve.add(format!("{}.taxId10Digit must be 10 digits", prefix));
    }
}

fn is_digits_len(s: &str, l: usize) -> bool {
    s.len() == l && s.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tax_info::*;

    fn minimal_tax_info() -> TaxInfo {
        let mut t = TaxInfo::default();
        t.payer.name = "Test Payer".to_string();
        t.payee.name = "Test Payee".to_string();
        t.payee.pnd_1a = true;
        t.withholding_type.withholding_tax = true;
        t
    }

    #[test]
    fn test_valid_tax_info() {
        let t = minimal_tax_info();
        assert!(validate_tax_info(&t).is_ok());
    }

    #[test]
    fn test_missing_payer_name() {
        let mut t = minimal_tax_info();
        t.payer.name = "".to_string();
        let err = validate_tax_info(&t).unwrap_err();
        assert!(err.errors.iter().any(|e| e.contains("payer.name is required")));
    }

    #[test]
    fn test_invalid_tax_id() {
        let mut t = minimal_tax_info();
        t.payer.tax_id = "123".to_string(); // Too short
        let err = validate_tax_info(&t).unwrap_err();
        assert!(err.errors.iter().any(|e| e.contains("payer.taxId must be 13 digits")));
    }

    #[test]
    fn test_no_pnd_selected() {
        let mut t = minimal_tax_info();
        t.payee.pnd_1a = false;
        let err = validate_tax_info(&t).unwrap_err();
        assert!(!err.errors.is_empty());
    }

    #[test]
    fn test_no_withholding_type() {
        let mut t = minimal_tax_info();
        t.withholding_type.withholding_tax = false;
        let err = validate_tax_info(&t).unwrap_err();
        assert!(!err.errors.is_empty());
    }
}
