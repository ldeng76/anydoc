//! PDF via [pdf-inspector]: classification plus direct Markdown extraction.
//!
//! Unlike the other frontends, pdf-inspector emits Markdown itself, so PDFs
//! bypass the document model and the shared GFM writer. Scanned and
//! image-only PDFs need OCR, which is out of scope here; they error as
//! unsupported. Pages flagged for OCR in an otherwise text-based document
//! degrade with a log, consistent with the crate-wide recovery policy.
//!
//! [pdf-inspector]: https://github.com/firecrawl/pdf-inspector

use crate::error::ConvertError;
use pdf_inspector::PdfError;

/// How pdf-inspector classified the whole document before extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfType {
    /// Most sampled pages have extractable text; direct extraction works.
    TextBased,
    /// Images only, no text operators: a classic scan. Whole document needs OCR.
    Scanned,
    /// Mostly images with minimal or no text.
    ImageBased,
    /// A mix of text pages and image/scan pages; OCR is needed per page.
    Mixed,
}

impl From<pdf_inspector::PdfType> for PdfType {
    fn from(pdf_type: pdf_inspector::PdfType) -> Self {
        match pdf_type {
            pdf_inspector::PdfType::TextBased => PdfType::TextBased,
            pdf_inspector::PdfType::Scanned => PdfType::Scanned,
            pdf_inspector::PdfType::ImageBased => PdfType::ImageBased,
            pdf_inspector::PdfType::Mixed => PdfType::Mixed,
        }
    }
}

/// One page that needs OCR, and why. The page number is 1-indexed, matching
/// the page count in errors and [`PdfInspection::pages_needing_ocr`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfOcrPage {
    /// 1-indexed page number.
    pub page: u32,
    /// Machine-readable reason codes: `scanned`, `no_text`, `vector_text`,
    /// or `suspected_garbled_text`.
    pub reasons: Vec<String>,
}

/// Detection-only result for a PDF: whether OCR is recommended, which pages
/// need it, why, and how confident the classification is. Produced without
/// text extraction, so it is cheap enough to call before deciding how to
/// process a document.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PdfInspection {
    /// The routing signal: `true` when pdf-inspector recommends OCR. This is
    /// broader than [`Self::pages_needing_ocr`]: whole-document heuristics
    /// (newspaper layouts, template images) can recommend OCR even when no
    /// individual page is flagged.
    pub needs_ocr: bool,
    /// The document-level classification.
    pub pdf_type: PdfType,
    /// Total number of pages in the document.
    pub page_count: u32,
    /// 1-indexed pages that need OCR: empty for text-based documents, every
    /// page for scanned/image-only ones, and a subset for mixed documents.
    pub pages_needing_ocr: Vec<u32>,
    /// Per-page reasons for every page in [`Self::pages_needing_ocr`].
    pub ocr_reasons: Vec<PdfOcrPage>,
    /// Detection confidence, 0.0 to 1.0.
    pub confidence: f32,
}

/// Inspect a PDF file without extracting text: returns `None` when the file
/// is not a PDF, the inspection when it is, and a typed error when it is a
/// PDF that cannot be parsed (malformed or encrypted).
pub fn inspect_pdf(path: impl AsRef<std::path::Path>) -> Result<Option<PdfInspection>, ConvertError> {
    let bytes = std::fs::read(path)?;
    inspect_pdf_bytes(&bytes)
}

/// Inspect an in-memory PDF without extracting text: returns `None` when the
/// bytes are not a PDF (same silent behavior as [`crate::Format::from_bytes`]),
/// the inspection when they are, and a typed error when they are a PDF that
/// cannot be parsed (malformed or encrypted).
pub fn inspect_pdf_bytes(bytes: &[u8]) -> Result<Option<PdfInspection>, ConvertError> {
    if crate::Format::from_bytes(bytes) != Some(crate::Format::Pdf) {
        return Ok(None);
    }
    let result = pdf_inspector::detect_pdf_type_mem(bytes).map_err(map_error)?;
    Ok(Some(PdfInspection {
        needs_ocr: result.ocr_recommended,
        pdf_type: result.pdf_type.into(),
        page_count: result.page_count,
        pages_needing_ocr: result.pages_needing_ocr,
        ocr_reasons: result
            .ocr_reasons_by_page
            .into_iter()
            .map(|(page, reasons)| PdfOcrPage { page, reasons })
            .collect(),
        confidence: result.confidence,
    }))
}

pub fn to_markdown(bytes: &[u8]) -> Result<String, ConvertError> {
    let result = pdf_inspector::process_pdf_mem(bytes).map_err(map_error)?;
    if !result.pages_needing_ocr.is_empty() {
        log::warn!(
            "{} of {} pages need OCR and were not extracted",
            result.pages_needing_ocr.len(),
            result.page_count
        );
    }
    if result.has_encoding_issues {
        log::warn!("broken font encodings detected; extracted text may be garbled");
    }
    match result.markdown {
        Some(mut markdown) if !markdown.trim().is_empty() => {
            if !markdown.ends_with('\n') {
                markdown.push('\n');
            }
            Ok(markdown)
        }
        _ => Err(ConvertError::Unsupported(format!(
            "PDF has no extractable text ({:?}, {} pages): OCR is required",
            result.pdf_type, result.page_count
        ))),
    }
}

fn map_error(e: PdfError) -> ConvertError {
    match e {
        PdfError::Encrypted => ConvertError::Encrypted,
        PdfError::Io(e) => ConvertError::Io(e),
        PdfError::NotAPdf(detail) => ConvertError::malformed(format!("not a PDF: {detail}")),
        PdfError::InvalidStructure => ConvertError::malformed("invalid PDF structure"),
        PdfError::Parse(detail) => ConvertError::malformed(detail),
    }
}
