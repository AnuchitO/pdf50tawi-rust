// REST server with 3 endpoints:
//   POST /api/v1/taxes/multipart — multipart/form-data
//   POST /api/v1/taxes/base64   — JSON with base64 images
//   POST /api/v1/taxes/url      — JSON with image URLs

use axum::{
    extract::Multipart,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

use pdf50tawi::{
    issue_wht_certificate_pdf, validate_tax_info,
    load_image_from_url,
    TaxInfo,
};

#[derive(Clone)]
struct AppState;

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let app = Router::new()
        .route("/api/v1/taxes/multipart", post(handle_multipart))
        .route("/api/v1/taxes/base64", post(handle_base64))
        .route("/api/v1/taxes/url", post(handle_url))
        .with_state(AppState);

    println!("Starting server on port {}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// ── Strategy A: multipart/form-data ──────────────────────────────────────────
async fn handle_multipart(mut multipart: Multipart) -> Response {
    let mut tax_info_json: Option<String> = None;
    let mut signature_data: Option<Vec<u8>> = None;
    let mut seal_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "taxInfo" => {
                match field.text().await {
                    Ok(text) => tax_info_json = Some(text),
                    Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("read taxInfo: {}", e)),
                }
            }
            "signature" => {
                match field.bytes().await {
                    Ok(bytes) => signature_data = Some(bytes.to_vec()),
                    Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("read signature: {}", e)),
                }
            }
            "seal" => {
                match field.bytes().await {
                    Ok(bytes) => seal_data = Some(bytes.to_vec()),
                    Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("read seal: {}", e)),
                }
            }
            _ => {}
        }
    }

    let json = match tax_info_json {
        Some(j) => j,
        None => return error_response(StatusCode::BAD_REQUEST, "missing 'taxInfo' form field"),
    };

    let tax_info: TaxInfo = match serde_json::from_str(&json) {
        Ok(t) => t,
        Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("invalid taxInfo JSON: {}", e)),
    };

    if let Err(e) = validate_tax_info(&tax_info) {
        return error_response(StatusCode::BAD_REQUEST, &e.to_string());
    }

    stream_certificate(tax_info, signature_data, seal_data)
}

// ── Strategy B: JSON with base64-encoded images ───────────────────────────────
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Base64Request {
    tax_info: TaxInfo,
    signature_base64: Option<String>,
    seal_base64: Option<String>,
}

async fn handle_base64(Json(req): Json<Base64Request>) -> Response {
    if let Err(e) = validate_tax_info(&req.tax_info) {
        return error_response(StatusCode::BAD_REQUEST, &e.to_string());
    }

    let sign_data = if let Some(b64) = &req.signature_base64 {
        match BASE64.decode(b64) {
            Ok(d) => Some(d),
            Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("invalid signatureBase64: {}", e)),
        }
    } else {
        None
    };

    let seal_data = if let Some(b64) = &req.seal_base64 {
        match BASE64.decode(b64) {
            Ok(d) => Some(d),
            Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("invalid sealBase64: {}", e)),
        }
    } else {
        None
    };

    stream_certificate(req.tax_info, sign_data, seal_data)
}

// ── Strategy C: JSON with image URLs ──────────────────────────────────────────
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UrlRequest {
    tax_info: TaxInfo,
    signature_url: Option<String>,
    seal_url: Option<String>,
}

async fn handle_url(Json(req): Json<UrlRequest>) -> Response {
    if let Err(e) = validate_tax_info(&req.tax_info) {
        return error_response(StatusCode::BAD_REQUEST, &e.to_string());
    }

    let sign_data = if let Some(url) = &req.signature_url {
        match load_image_from_url(url) {
            Ok(d) => Some(d),
            Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("signatureURL: {}", e)),
        }
    } else {
        None
    };

    let seal_data = if let Some(url) = &req.seal_url {
        match load_image_from_url(url) {
            Ok(d) => Some(d),
            Err(e) => return error_response(StatusCode::BAD_REQUEST, &format!("sealURL: {}", e)),
        }
    } else {
        None
    };

    stream_certificate(req.tax_info, sign_data, seal_data)
}

// ── Shared helpers ─────────────────────────────────────────────────────────────

fn stream_certificate(tax_info: TaxInfo, sign: Option<Vec<u8>>, seal: Option<Vec<u8>>) -> Response {
    let mut buf = Vec::new();
    match issue_wht_certificate_pdf(&mut buf, tax_info, sign, seal) {
        Ok(()) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/pdf"),
                (header::CONTENT_DISPOSITION, "attachment; filename=certificate.pdf"),
            ],
            buf,
        ).into_response(),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("generate certificate: {}", e)),
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

fn error_response(status: StatusCode, msg: &str) -> Response {
    (status, Json(ErrorBody { error: msg.to_string() })).into_response()
}
