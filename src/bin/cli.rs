// CLI binary — images supplied as local file paths via flags.
//
// Usage:
//   cli --signature path/to/signature.png --seal path/to/seal.png --output certificate.pdf

use clap::Parser;
use std::fs;
use pdf50tawi::{
    issue_wht_certificate_pdf, validate_tax_info,
    load_image_from_file,
    TaxInfo,
};

#[derive(Parser)]
#[command(name = "cli", about = "Generate Thai WHT Certificate PDF")]
struct Args {
    /// Output PDF file path
    #[arg(long, default_value = "certificate.pdf")]
    output: String,

    /// Signature image file path (PNG)
    #[arg(long, default_value = "")]
    signature: String,

    /// Company seal image file path (PNG)
    #[arg(long, default_value = "")]
    seal: String,
}

fn main() {
    let args = Args::parse();

    let tax_info = demo_tax_info();

    if let Err(e) = validate_tax_info(&tax_info) {
        eprintln!("validation error: {}", e);
        std::process::exit(1);
    }

    let sign = load_optional(&args.signature, "signature");
    let seal = load_optional(&args.seal, "seal");

    let mut out = fs::File::create(&args.output)
        .unwrap_or_else(|e| {
            eprintln!("create output: {}", e);
            std::process::exit(1);
        });

    if let Err(e) = issue_wht_certificate_pdf(&mut out, tax_info, sign, seal) {
        eprintln!("generate certificate: {}", e);
        std::process::exit(1);
    }

    println!("Certificate written to {}", args.output);
}

fn load_optional(path: &str, label: &str) -> Option<Vec<u8>> {
    if path.is_empty() {
        return None;
    }
    match load_image_from_file(path) {
        Ok(data) => Some(data),
        Err(e) => {
            eprintln!("load {} ({}): {}", label, path, e);
            std::process::exit(1);
        }
    }
}

fn demo_tax_info() -> TaxInfo {
    use pdf50tawi::tax_info::*;
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
            total_tax_withheld_in_words: "หนึ่งแสนเจ็ดหมื่นสองพันสองร้อยสามสิบเก้าบาทหกสิบสตางค์".to_string(),
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
