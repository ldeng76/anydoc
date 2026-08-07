//! PDF OCR pre-check, serialized to a plain JS object. The shape mirrors the
//! Node bindings; the TypeScript definitions live in `typescript.rs`.

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PdfType {
    TextBased,
    Scanned,
    ImageBased,
    Mixed,
}

#[derive(Serialize)]
pub struct PdfOcrPage {
    pub page: u32,
    pub reasons: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfInspection {
    pub needs_ocr: bool,
    pub pdf_type: PdfType,
    pub page_count: u32,
    pub pages_needing_ocr: Vec<u32>,
    pub ocr_reasons: Vec<PdfOcrPage>,
    pub confidence: f64,
}

impl From<anydoc::PdfInspection> for PdfInspection {
    fn from(inspection: anydoc::PdfInspection) -> Self {
        PdfInspection {
            needs_ocr: inspection.needs_ocr,
            pdf_type: match inspection.pdf_type {
                anydoc::PdfType::TextBased => PdfType::TextBased,
                anydoc::PdfType::Scanned => PdfType::Scanned,
                anydoc::PdfType::ImageBased => PdfType::ImageBased,
                anydoc::PdfType::Mixed => PdfType::Mixed,
            },
            page_count: inspection.page_count,
            pages_needing_ocr: inspection.pages_needing_ocr,
            ocr_reasons: inspection
                .ocr_reasons
                .into_iter()
                .map(|page| PdfOcrPage { page: page.page, reasons: page.reasons })
                .collect(),
            confidence: inspection.confidence as f64,
        }
    }
}
