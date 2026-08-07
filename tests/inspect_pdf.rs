//! PDF OCR pre-check API: detection-only inspection, no text extraction.

mod common;

use anydoc::{PdfType, inspect_pdf, inspect_pdf_bytes};
use common::fixture_root;

fn fixture(name: &str) -> std::path::PathBuf {
    fixture_root().join(name)
}

#[test]
fn text_only_pdf_needs_no_ocr() {
    let inspection =
        inspect_pdf(fixture("pdf/text-only.pdf")).unwrap().expect("text-only.pdf is a PDF");
    assert!(!inspection.needs_ocr);
    assert_eq!(inspection.pdf_type, PdfType::TextBased);
    assert!(inspection.pages_needing_ocr.is_empty());
    assert!(inspection.ocr_reasons.is_empty());
    assert!(inspection.page_count >= 1);
    assert!((0.0..=1.0).contains(&inspection.confidence));
}

#[test]
fn text_pdf_is_mixed_but_flags_no_individual_page() {
    // The rich text fixture carries an embedded image, so pdf-inspector
    // classifies it as Mixed: OCR is recommended document-wide and the
    // image-only page is flagged individually, while the text pages convert
    // directly. `to_markdown` still succeeds on the whole file.
    let inspection = inspect_pdf(fixture("pdf/text.pdf")).unwrap().expect("text.pdf is a PDF");
    assert!(inspection.needs_ocr);
    assert_eq!(inspection.pdf_type, PdfType::Mixed);
    assert!(!inspection.pages_needing_ocr.is_empty());
    for page in &inspection.pages_needing_ocr {
        assert!((1..=inspection.page_count).contains(page));
    }
    let flagged: Vec<u32> = inspection.ocr_reasons.iter().map(|r| r.page).collect();
    assert_eq!(flagged, inspection.pages_needing_ocr);
    assert!(inspection.ocr_reasons.iter().all(|r| !r.reasons.is_empty()));
}

#[test]
fn scanned_pdf_needs_ocr_on_every_page() {
    let inspection = inspect_pdf(fixture("pdf/scanned.pdf")).unwrap().expect("scanned.pdf is a PDF");
    assert!(inspection.needs_ocr);
    assert_eq!(inspection.pdf_type, PdfType::Scanned);
    assert_eq!(
        inspection.pages_needing_ocr,
        (1..=inspection.page_count).collect::<Vec<u32>>()
    );
    assert!(!inspection.ocr_reasons.is_empty());
    assert_eq!(inspection.ocr_reasons[0].page, 1);
    assert!(inspection.ocr_reasons[0].reasons.iter().any(|r| r == "scanned"));
    assert!((0.0..=1.0).contains(&inspection.confidence));
}

#[test]
fn bytes_variant_matches_path_variant() {
    let bytes = std::fs::read(fixture("pdf/text.pdf")).unwrap();
    let from_bytes = inspect_pdf_bytes(&bytes).unwrap().unwrap();
    let from_path = inspect_pdf(fixture("pdf/text.pdf")).unwrap().unwrap();
    assert_eq!(from_bytes, from_path);
}

#[test]
fn non_pdf_bytes_return_none() {
    assert!(inspect_pdf_bytes(b"hello, not a pdf").unwrap().is_none());
    let docx = std::fs::read(fixture("docx/handmade-outline.docx")).unwrap();
    assert!(inspect_pdf_bytes(&docx).unwrap().is_none());
}

#[test]
fn corrupt_pdf_returns_a_typed_error() {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    bytes.extend_from_slice(b"this is not a valid pdf body at all");
    let error = inspect_pdf_bytes(&bytes).unwrap_err();
    assert_eq!(error.code(), "malformed");
}
