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
            book_number: "1".to_string(),
            document_number: "2568001".to_string(),
        },
        payer: Payer {
            tax_id: "1234567890123".to_string(),
            tax_id10_digit: "".to_string(),
            name: "บริษัท ตัวอย่าง จำกัด".to_string(),
            address: "123 ถนนสุขุมวิท แขวงคลองเตย เขตคลองเตย กรุงเทพมหานคร 10110".to_string(),
        },
        payee: Payee {
            tax_id: "9876543210987".to_string(),
            tax_id10_digit: "".to_string(),
            name: "นาย ทดสอบ ระบบ".to_string(),
            address: "456 ถนนพระราม 4 แขวงสีลม เขตบางรัก กรุงเทพมหานคร 10500".to_string(),
            sequence_number: "1".to_string(),
            pnd_1a: true,
            pnd_1a_special: false,
            pnd_2: false,
            pnd_3: false,
            pnd_2a: false,
            pnd_3a: false,
            pnd_53: false,
        },
        income40_1: IncomeDetail {
            date_paid: "15/01/2568".to_string(),
            amount_paid: "50,000.00".to_string(),
            tax_withheld: "2,500.00".to_string(),
        },
        withholding_type: WithholdingType {
            withholding_tax: true,
            forever: false,
            one_time: false,
            other: false,
            other_details: "".to_string(),
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
