//! PDF OCR pre-check bindings.
//!
//! `pdf_type` surfaces as a camelCase string (`textBased`, `scanned`,
//! `imageBased`, `mixed`), the same vocabulary the Node bindings use.

use pyo3::prelude::*;
use pyo3::types::PyList;

/// One page that needs OCR, and why.
#[pyclass(frozen, get_all, module = "anydoc")]
pub struct PdfOcrPage {
    /// 1-indexed page number.
    page: u32,
    /// Machine-readable reason codes: `scanned`, `no_text`, `vector_text`,
    /// or `suspected_garbled_text`.
    reasons: Vec<String>,
}

/// Detection-only result for a PDF: whether OCR is recommended, which pages
/// need it, why, and how confident the classification is. Produced without
/// text extraction, so it is cheap enough to call before converting.
#[pyclass(frozen, get_all, module = "anydoc")]
pub struct PdfInspection {
    /// The routing signal: `True` when OCR is recommended. This is broader
    /// than `pages_needing_ocr`: whole-document heuristics (newspaper
    /// layouts, template images) can recommend OCR even when no page is
    /// flagged.
    needs_ocr: bool,
    /// The document-level classification: `textBased`, `scanned`,
    /// `imageBased`, or `mixed`.
    pdf_type: &'static str,
    /// Total number of pages in the document.
    page_count: u32,
    /// 1-indexed pages that need OCR.
    pages_needing_ocr: Vec<u32>,
    /// Per-page reasons for every page in `pages_needing_ocr`.
    /// list[PdfOcrPage]
    ocr_reasons: Py<PyList>,
    /// Detection confidence, 0.0 to 1.0.
    confidence: f64,
}

fn pdf_type_name(pdf_type: anydoc::PdfType) -> &'static str {
    match pdf_type {
        anydoc::PdfType::TextBased => "textBased",
        anydoc::PdfType::Scanned => "scanned",
        anydoc::PdfType::ImageBased => "imageBased",
        anydoc::PdfType::Mixed => "mixed",
    }
}

pub(crate) fn inspection(
    py: Python<'_>,
    inspection: anydoc::PdfInspection,
) -> PyResult<PdfInspection> {
    let ocr_reasons = inspection
        .ocr_reasons
        .into_iter()
        .map(|page| Py::new(py, PdfOcrPage { page: page.page, reasons: page.reasons }))
        .collect::<PyResult<Vec<_>>>()?;
    Ok(PdfInspection {
        needs_ocr: inspection.needs_ocr,
        pdf_type: pdf_type_name(inspection.pdf_type),
        page_count: inspection.page_count,
        pages_needing_ocr: inspection.pages_needing_ocr,
        ocr_reasons: PyList::new(py, ocr_reasons)?.unbind(),
        confidence: inspection.confidence as f64,
    })
}
