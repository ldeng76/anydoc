//! Node.js bindings for anydoc.

use napi::bindgen_prelude::*;
use napi_derive::napi;

mod document;

pub use document::*;

/// Input format, named after the extension that identifies it. Container
/// variants that share a parser (`.docm`, `.xlsm`, `.ppsx`, ...) map onto
/// these.
#[napi(string_enum)]
#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum Format {
    doc,
    docx,
    odt,
    /// Converted with pdf-inspector, which emits Markdown directly:
    /// `toDocument` is unsupported for PDFs. Scanned or image-only PDFs
    /// (needing OCR) error as unsupported.
    pdf,
    ppt,
    pptx,
    rtf,
    epub,
    xlsx,
    ods,
    odp,
    csv,
}

impl From<Format> for anydoc::Format {
    fn from(format: Format) -> Self {
        match format {
            Format::doc => anydoc::Format::Doc,
            Format::docx => anydoc::Format::Docx,
            Format::odt => anydoc::Format::Odt,
            Format::pdf => anydoc::Format::Pdf,
            Format::ppt => anydoc::Format::Ppt,
            Format::pptx => anydoc::Format::Pptx,
            Format::rtf => anydoc::Format::Rtf,
            Format::epub => anydoc::Format::Epub,
            Format::xlsx => anydoc::Format::Excel,
            Format::ods => anydoc::Format::Ods,
            Format::odp => anydoc::Format::Odp,
            Format::csv => anydoc::Format::Csv,
        }
    }
}

impl From<anydoc::Format> for Format {
    fn from(format: anydoc::Format) -> Self {
        match format {
            anydoc::Format::Doc => Format::doc,
            anydoc::Format::Docx => Format::docx,
            anydoc::Format::Odt => Format::odt,
            anydoc::Format::Pdf => Format::pdf,
            anydoc::Format::Ppt => Format::ppt,
            anydoc::Format::Pptx => Format::pptx,
            anydoc::Format::Rtf => Format::rtf,
            anydoc::Format::Epub => Format::epub,
            anydoc::Format::Excel => Format::xlsx,
            anydoc::Format::Ods => Format::ods,
            anydoc::Format::Odp => Format::odp,
            anydoc::Format::Csv => Format::csv,
        }
    }
}

/// Detect the format from the content itself: the signature and identity each
/// container specification designates (PDF header, RTF open group, OLE stream
/// names, ZIP package mimetype/content types). Plain-text formats (CSV) carry
/// no signature and return `null`; so does anything unrecognized.
#[napi]
pub fn format_from_bytes(bytes: Uint8Array) -> Option<Format> {
    anydoc::Format::from_bytes(&bytes).map(Format::from)
}

/// The format an extension names, with or without a leading dot.
#[napi]
pub fn format_from_extension(extension: String) -> Option<Format> {
    anydoc::Format::from_extension(extension.trim_start_matches('.')).map(Format::from)
}

/// The format a path's extension names.
#[napi]
pub fn format_from_path(path: String) -> Option<Format> {
    anydoc::Format::from_path(std::path::Path::new(&path)).map(Format::from)
}

/// Convert a document file to Markdown. The format is detected from the file
/// content; the extension is the fallback for signature-less formats (CSV)
/// and unrecognizable containers.
///
/// Rejects with an `Error` carrying a `ConvertErrorCode` on `code`; a file
/// that cannot be read is `'io'`.
#[napi(ts_return_type = "Promise<string>")]
pub fn to_markdown(path: String) -> AsyncTask<MarkdownFileTask> {
    AsyncTask::new(MarkdownFileTask { path, failure: Failure::default() })
}

/// Convert an in-memory document to Markdown. Without a format, it is
/// detected from the content, which signature-less formats (CSV) have to name
/// explicitly.
///
/// Rejects with an `Error` carrying a `ConvertErrorCode` on `code`.
#[napi(ts_return_type = "Promise<string>")]
pub fn to_markdown_bytes(
    bytes: Uint8Array,
    format: Option<Format>,
) -> AsyncTask<MarkdownBytesTask> {
    AsyncTask::new(MarkdownBytesTask {
        bytes: bytes.to_vec(),
        format: format.map(Into::into),
        failure: Failure::default(),
    })
}

/// Parse an in-memory document into the document model, which also carries
/// the embedded assets. Without a format, it is detected from the content.
///
/// Unsupported for `pdf`: PDF conversion produces Markdown directly and has
/// no document-model form; use `toMarkdownBytes`.
///
/// Rejects with an `Error` carrying a `ConvertErrorCode` on `code`.
#[napi(ts_return_type = "Promise<Document>")]
pub fn to_document(bytes: Uint8Array, format: Option<Format>) -> AsyncTask<DocumentTask> {
    AsyncTask::new(DocumentTask {
        bytes: bytes.to_vec(),
        format: format.map(Into::into),
        failure: Failure::default(),
    })
}

/// How pdf-inspector classified the document before extraction.
#[napi(string_enum)]
#[allow(non_camel_case_types)]
pub enum PdfType {
    textBased,
    scanned,
    imageBased,
    mixed,
}

impl From<anydoc::PdfType> for PdfType {
    fn from(pdf_type: anydoc::PdfType) -> Self {
        match pdf_type {
            anydoc::PdfType::TextBased => PdfType::textBased,
            anydoc::PdfType::Scanned => PdfType::scanned,
            anydoc::PdfType::ImageBased => PdfType::imageBased,
            anydoc::PdfType::Mixed => PdfType::mixed,
        }
    }
}

/// One page that needs OCR, and why. `page` is 1-indexed.
#[napi(object)]
pub struct PdfOcrPage {
    /// 1-indexed page number.
    pub page: u32,
    /// Machine-readable reason codes: `scanned`, `no_text`, `vector_text`,
    /// or `suspected_garbled_text`.
    pub reasons: Vec<String>,
}

impl From<anydoc::PdfOcrPage> for PdfOcrPage {
    fn from(page: anydoc::PdfOcrPage) -> Self {
        PdfOcrPage { page: page.page, reasons: page.reasons }
    }
}

/// Detection-only result for a PDF: whether OCR is recommended, which pages
/// need it, why, and how confident the classification is. Produced without
/// text extraction, so it is cheap enough to call before converting.
#[napi(object)]
pub struct PdfInspection {
    /// The routing signal: `true` when OCR is recommended. This is broader
    /// than `pagesNeedingOcr`: whole-document heuristics (newspaper layouts,
    /// template images) can recommend OCR even when no page is flagged.
    pub needs_ocr: bool,
    /// The document-level classification.
    pub pdf_type: PdfType,
    /// Total number of pages in the document.
    pub page_count: u32,
    /// 1-indexed pages that need OCR.
    pub pages_needing_ocr: Vec<u32>,
    /// Per-page reasons for every page in `pagesNeedingOcr`.
    pub ocr_reasons: Vec<PdfOcrPage>,
    /// Detection confidence, 0.0 to 1.0.
    pub confidence: f64,
}

impl From<anydoc::PdfInspection> for PdfInspection {
    fn from(inspection: anydoc::PdfInspection) -> Self {
        PdfInspection {
            needs_ocr: inspection.needs_ocr,
            pdf_type: inspection.pdf_type.into(),
            page_count: inspection.page_count,
            pages_needing_ocr: inspection.pages_needing_ocr,
            ocr_reasons: inspection.ocr_reasons.into_iter().map(Into::into).collect(),
            confidence: inspection.confidence as f64,
        }
    }
}

/// Inspect a PDF file without extracting text: resolves to `null` when the
/// file is not a PDF, the inspection when it is, and rejects with a
/// `ConvertErrorCode` when it is a PDF that cannot be parsed.
#[napi(ts_return_type = "Promise<PdfInspection | null>")]
pub fn inspect_pdf(path: String) -> AsyncTask<InspectPdfFileTask> {
    AsyncTask::new(InspectPdfFileTask { path, failure: Failure::default() })
}

/// Inspect an in-memory PDF without extracting text: resolves to `null` when
/// the bytes are not a PDF, the inspection when they are, and rejects with a
/// `ConvertErrorCode` when they are a PDF that cannot be parsed.
#[napi(ts_return_type = "Promise<PdfInspection | null>")]
pub fn inspect_pdf_bytes(bytes: Uint8Array) -> AsyncTask<InspectPdfBytesTask> {
    AsyncTask::new(InspectPdfBytesTask {
        bytes: bytes.to_vec(),
        failure: Failure::default(),
    })
}

/// The kind of a failed conversion, held between the two threads a rejection
/// crosses: `compute` runs on the libuv pool, where there is no `Env` to build
/// a JS error with, and `reject` runs on the JS thread, where there is.
#[derive(Default)]
struct Failure(Option<&'static str>);

impl Failure {
    /// Keep the kind, and hand napi the message to reject with.
    fn capture(&mut self, error: anydoc::ConvertError) -> Error {
        self.0 = Some(error.code());
        Error::from_reason(error.to_string())
    }

    /// Rebuild the rejection as an error whose `code` is the `ConvertError`
    /// kind. napi fills `code` from the error's status, so the status here is
    /// a plain string rather than the `Status` enum it defaults to. Anything
    /// that did not come from `capture` is napi's own failure: pass it on.
    fn reject(&self, env: Env, error: Error) -> Error {
        let Some(code) = self.0 else {
            return error;
        };
        let coded = Error::new(code.to_owned(), error.reason.clone());
        Error::from(JsError::from(coded).into_unknown(env))
    }
}

pub struct MarkdownFileTask {
    path: String,
    failure: Failure,
}

impl Task for MarkdownFileTask {
    type Output = String;
    type JsValue = String;

    fn compute(&mut self) -> Result<Self::Output> {
        anydoc::to_markdown(&self.path).map_err(|e| self.failure.capture(e))
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }

    fn reject(&mut self, env: Env, error: Error) -> Result<Self::JsValue> {
        Err(self.failure.reject(env, error))
    }
}

pub struct MarkdownBytesTask {
    bytes: Vec<u8>,
    format: Option<anydoc::Format>,
    failure: Failure,
}

impl Task for MarkdownBytesTask {
    type Output = String;
    type JsValue = String;

    fn compute(&mut self) -> Result<Self::Output> {
        anydoc::to_markdown_bytes(&self.bytes, self.format).map_err(|e| self.failure.capture(e))
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }

    fn reject(&mut self, env: Env, error: Error) -> Result<Self::JsValue> {
        Err(self.failure.reject(env, error))
    }
}

pub struct DocumentTask {
    bytes: Vec<u8>,
    format: Option<anydoc::Format>,
    failure: Failure,
}

impl Task for DocumentTask {
    type Output = anydoc::model::Document;
    type JsValue = Document;

    fn compute(&mut self) -> Result<Self::Output> {
        anydoc::to_document(&self.bytes, self.format).map_err(|e| self.failure.capture(e))
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output.into())
    }

    fn reject(&mut self, env: Env, error: Error) -> Result<Self::JsValue> {
        Err(self.failure.reject(env, error))
    }
}

pub struct InspectPdfFileTask {
    path: String,
    failure: Failure,
}

impl Task for InspectPdfFileTask {
    type Output = Option<anydoc::PdfInspection>;
    type JsValue = Option<PdfInspection>;

    fn compute(&mut self) -> Result<Self::Output> {
        anydoc::inspect_pdf(&self.path).map_err(|e| self.failure.capture(e))
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output.map(Into::into))
    }

    fn reject(&mut self, env: Env, error: Error) -> Result<Self::JsValue> {
        Err(self.failure.reject(env, error))
    }
}

pub struct InspectPdfBytesTask {
    bytes: Vec<u8>,
    failure: Failure,
}

impl Task for InspectPdfBytesTask {
    type Output = Option<anydoc::PdfInspection>;
    type JsValue = Option<PdfInspection>;

    fn compute(&mut self) -> Result<Self::Output> {
        anydoc::inspect_pdf_bytes(&self.bytes).map_err(|e| self.failure.capture(e))
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output.map(Into::into))
    }

    fn reject(&mut self, env: Env, error: Error) -> Result<Self::JsValue> {
        Err(self.failure.reject(env, error))
    }
}
