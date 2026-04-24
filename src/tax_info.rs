use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxInfo {
    pub document_details: DocumentDetails,
    pub payer: Payer,
    pub payee: Payee,

    pub income40_1: IncomeDetail,
    pub income40_2: IncomeDetail,
    pub income40_3: IncomeDetail,
    pub income40_4a: IncomeDetail,

    pub income40_4b_1_1: IncomeDetail,
    pub income40_4b_1_2: IncomeDetail,
    pub income40_4b_1_3: IncomeDetail,
    #[serde(rename = "income40_4B_1_4_rate")]
    pub income40_4b_1_4_rate: String,
    pub income40_4b_1_4: IncomeDetail,
    pub income40_4b_2_1: IncomeDetail,
    pub income40_4b_2_2: IncomeDetail,
    pub income40_4b_2_3: IncomeDetail,
    pub income40_4b_2_4: IncomeDetail,
    #[serde(rename = "income40_4B_2_5_note")]
    pub income40_4b_2_5_note: String,
    pub income40_4b_2_5: IncomeDetail,

    pub income5: IncomeDetail,
    pub income6: IncomeDetail,
    pub income6_note: String,

    pub totals: Totals,
    pub other_payments: OtherPayments,
    pub withholding_type: WithholdingType,
    pub certification: Certification,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDetails {
    pub book_number: String,
    pub document_number: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payer {
    pub tax_id: String,
    pub tax_id10_digit: String,
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payee {
    pub tax_id: String,
    pub tax_id10_digit: String,
    pub name: String,
    pub address: String,
    pub sequence_number: String,
    pub pnd_1a: bool,
    pub pnd_1a_special: bool,
    pub pnd_2: bool,
    pub pnd_3: bool,
    pub pnd_2a: bool,
    pub pnd_3a: bool,
    pub pnd_53: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomeDetail {
    pub date_paid: String,
    pub amount_paid: String,
    pub tax_withheld: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub total_amount_paid: String,
    pub total_tax_withheld: String,
    pub total_tax_withheld_in_words: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtherPayments {
    pub government_pension_fund: String,
    pub social_security_fund: String,
    pub provident_fund: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithholdingType {
    pub withholding_tax: bool,
    pub forever: bool,
    pub one_time: bool,
    pub other: bool,
    pub other_details: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DateOfIssuance {
    pub day: String,
    pub month: String,
    pub year: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Certification {
    pub date_of_issuance: DateOfIssuance,
}
