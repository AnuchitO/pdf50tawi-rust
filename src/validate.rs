use crate::tax_info::{Payee, TaxInfo, WithholdingType};
use thiserror::Error;

#[derive(Debug, Default, Error)]
#[error("{}", errors.join("; "))]
pub struct ValidationError {
    pub errors: Vec<String>,
}

impl ValidationError {
    fn add(&mut self, msg: impl Into<String>) {
        self.errors.push(msg.into());
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Validates all fields in [`TaxInfo`] and returns a combined error if any fail.
pub fn validate_tax_info(t: &TaxInfo) -> Result<(), ValidationError> {
    let mut ve = ValidationError::default();

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

    fn minimal() -> TaxInfo {
        let mut t = TaxInfo::default();
        t.payer.name = "Test Payer".to_string();
        t.payee.name = "Test Payee".to_string();
        t.payee.pnd_1a = true;
        t.withholding_type.withholding_tax = true;
        t
    }

    #[test]
    fn valid_minimal_passes() {
        assert!(validate_tax_info(&minimal()).is_ok());
    }

    #[test]
    fn empty_payer_name_is_rejected() {
        let mut t = minimal();
        t.payer.name = String::new();
        let err = validate_tax_info(&t).unwrap_err();
        assert!(err.errors.iter().any(|e| e.contains("payer.name is required")));
    }

    #[test]
    fn whitespace_only_name_is_rejected() {
        let mut t = minimal();
        t.payer.name = "   ".to_string();
        let err = validate_tax_info(&t).unwrap_err();
        assert!(err.errors.iter().any(|e| e.contains("payer.name is required")));
    }

    #[test]
    fn short_tax_id_is_rejected() {
        let mut t = minimal();
        t.payer.tax_id = "123".to_string();
        let err = validate_tax_info(&t).unwrap_err();
        assert!(err.errors.iter().any(|e| e.contains("payer.taxId must be 13 digits")));
    }

    #[test]
    fn tax_id_with_spaces_is_accepted() {
        let mut t = minimal();
        t.payer.tax_id = "1 234 5678 9012 3".to_string(); // 13 digits with spaces
        assert!(validate_tax_info(&t).is_ok());
    }

    #[test]
    fn invalid_10digit_tax_id_is_rejected() {
        let mut t = minimal();
        t.payer.tax_id10_digit = "12345".to_string();
        let err = validate_tax_info(&t).unwrap_err();
        assert!(err.errors.iter().any(|e| e.contains("payer.taxId10Digit must be 10 digits")));
    }

    #[test]
    fn no_pnd_selected_is_rejected() {
        let mut t = minimal();
        t.payee.pnd_1a = false;
        assert!(validate_tax_info(&t).is_err());
    }

    #[test]
    fn any_pnd_type_is_sufficient() {
        for (i, pnd) in ["pnd_1a", "pnd_1a_special", "pnd_2", "pnd_3", "pnd_2a", "pnd_3a", "pnd_53"].iter().enumerate() {
            let mut t = minimal();
            t.payee.pnd_1a = false;
            match i {
                0 => t.payee.pnd_1a = true,
                1 => t.payee.pnd_1a_special = true,
                2 => t.payee.pnd_2 = true,
                3 => t.payee.pnd_3 = true,
                4 => t.payee.pnd_2a = true,
                5 => t.payee.pnd_3a = true,
                6 => t.payee.pnd_53 = true,
                _ => unreachable!(),
            }
            assert!(validate_tax_info(&t).is_ok(), "{} should be valid", pnd);
        }
    }

    #[test]
    fn no_withholding_type_is_rejected() {
        let mut t = minimal();
        t.withholding_type.withholding_tax = false;
        assert!(validate_tax_info(&t).is_err());
    }

    #[test]
    fn multiple_errors_are_collected() {
        let t = TaxInfo::default(); // all empty/false
        let err = validate_tax_info(&t).unwrap_err();
        // At minimum: payer.name, payee.name, no pnd, no withholding type
        assert!(err.errors.len() >= 4);
    }

    #[test]
    fn error_display_joins_messages() {
        let mut t = minimal();
        t.payer.name = String::new();
        t.payee.name = String::new();
        let err = validate_tax_info(&t).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("payer.name is required"));
        assert!(msg.contains("payee.name is required"));
        assert!(msg.contains(';')); // joined with "; "
    }
}
