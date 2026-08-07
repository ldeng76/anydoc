//! TypeScript definitions wasm-bindgen cannot generate on its own.
//!
//! `toDocument` returns plain JS objects built with serde, so the declarations
//! here mirror the shapes produced by `document.rs` and must be kept in step
//! with it. The `code` on a thrown error is out of wasm-bindgen's reach the
//! same way; it mirrors `ConvertError::code` in the crate.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TYPESCRIPT: &str = r#"
/**
 * `code` on the `Error` a failed conversion throws. Conversion fails only
 * when no meaningful Markdown could be produced; producer quirks are
 * recovered or skipped instead. The crate's `io` code has no wasm
 * counterpart: there is no filesystem to read from.
 */
export type ConvertErrorCode =
  /** Unknown format, or one that cannot be converted (an image-only PDF). */
  | 'unsupported'
  /** Structurally unusable: no meaningful content could be extracted. */
  | 'malformed'
  /** Encrypted or password-protected. */
  | 'encrypted'
  /** Crossed a fixed safety limit (decompression, nesting, node count). */
  | 'resourceLimit'
  /** A part required for any meaningful output is absent. */
  | 'missingPart'

export interface Document {
  blocks: Array<Block>
  /**
   * Footnote and endnote bodies, referenced from text by a `noteRef`
   * inline.
   */
  notes: Array<Note>
  assets: Array<Asset>
}

export type BlockKind =
  | 'heading'
  | 'paragraph'
  | 'list'
  | 'table'
  | 'blockQuote'
  | 'codeBlock'
  | 'rule'

export interface Block {
  kind: BlockKind
  /** heading: 1-6. */
  level?: number
  /** heading: stable anchor id when the document targets this heading. */
  anchor?: string
  /** heading, paragraph. */
  content?: Array<Inline>
  list?: List
  table?: Table
  /** blockQuote. */
  blocks?: Array<Block>
  /** codeBlock. */
  lang?: string
  /** codeBlock. */
  text?: string
}

export type InlineKind =
  | 'text'
  | 'link'
  | 'image'
  /** Zero-width marker for an internal link target at this position. */
  | 'anchor'
  | 'noteRef'
  | 'lineBreak'

export interface Inline {
  kind: InlineKind
  /** text. */
  text?: string
  /** text. */
  style?: Style
  /** link. */
  content?: Array<Inline>
  /** link. */
  target?: LinkTarget
  /** image. */
  alt?: string
  /** image. */
  source?: ImageSource
  /** anchor: the anchor id. */
  anchor?: string
  /** noteRef: the id of the note in `Document.notes`. */
  noteId?: string
}

/** Fully resolved character style. */
export interface Style {
  bold: boolean
  italic: boolean
  strike: boolean
  code: boolean
}

export type LinkTargetKind =
  /** Absolute URL with a scheme. */
  | 'external'
  /** Scheme-less relative reference, preserved as written. */
  | 'relative'
  /** Internal target: a heading anchor or an `anchor` inline. */
  | 'anchor'

export interface LinkTarget {
  kind: LinkTargetKind
  /** The URL, relative reference, or anchor id. */
  value: string
}

export type ImageSourceKind =
  /** Absolute URL with a scheme. */
  | 'external'
  /** Embedded image, carried in `Document.assets`. */
  | 'asset'
  /**
   * No usable source: the image's part is missing or unreadable and it has
   * no URL. Only the alt text remains.
   */
  | 'unavailable'

export interface ImageSource {
  kind: ImageSourceKind
  /** external. */
  url?: string
  /** asset: index into `Document.assets`. */
  assetId?: number
}

/** The marker family a list uses in the source document. */
export type MarkerKind =
  | 'bullet'
  | 'decimal'
  | 'lowerAlpha'
  | 'upperAlpha'
  | 'lowerRoman'
  | 'upperRoman'

export interface List {
  marker: MarkerKind
  /** Ordinal the first item counts from. */
  start: number
  items: Array<ListItem>
}

export interface ListItem {
  blocks: Array<Block>
  /** Task-list state, when the item carries a checkbox. */
  checked?: boolean
  /**
   * Literal marker text that overrides the list marker when the source
   * number text cannot be reproduced from the marker and position alone
   * (composite number text such as `1-a)`).
   */
  markerLabel?: string
}

export type TableKind =
  /** A real data table. */
  | 'data'
  /** Layout scaffolding (text boxes, positioning tables). */
  | 'layout'

/**
 * Canonical table grid: every logical grid position appears exactly once.
 * Content and spans live on the origin slot, and each position a span covers
 * holds a `covered` slot pointing back at that origin.
 */
export interface Table {
  grid: Array<Array<CellSlot>>
  /** Number of leading rows that are header rows (0 = no header). */
  headerRows: number
  kind: TableKind
}

export type CellSlotKind = 'origin' | 'covered'

export interface CellSlot {
  kind: CellSlotKind
  /** origin. */
  cell?: Cell
  /** covered: row of the origin this position belongs to. */
  originRow?: number
  /** covered: column of the origin this position belongs to. */
  originCol?: number
}

export interface Cell {
  blocks: Array<Block>
  colSpan: number
  rowSpan: number
}

export type NoteKind = 'footnote' | 'endnote'

export interface Note {
  id: string
  kind: NoteKind
  blocks: Array<Block>
}

/**
 * An embedded binary asset (image, object payload). Bytes are always
 * retained, so a document stays self-contained.
 */
export interface Asset {
  /** Index into `Document.assets`, as referenced by an image source. */
  id: number
  /** MIME type, e.g. `image/png`. */
  mediaType: string
  /** Package part or stream the asset came from, for provenance. */
  originPart: string
  data: Uint8Array
}

export type PdfType = 'textBased' | 'scanned' | 'imageBased' | 'mixed'

/**
 * Detection-only result for a PDF: whether OCR is recommended, which pages
 * need it, why, and how confident the classification is. Produced without
 * text extraction, so it is cheap enough to call before converting.
 */
export interface PdfInspection {
  /**
   * The routing signal: `true` when OCR is recommended. This is broader
   * than `pagesNeedingOcr`: whole-document heuristics (newspaper layouts,
   * template images) can recommend OCR even when no page is flagged.
   */
  needsOcr: boolean
  /** The document-level classification. */
  pdfType: PdfType
  /** Total number of pages in the document. */
  pageCount: number
  /** 1-indexed pages that need OCR. */
  pagesNeedingOcr: Array<number>
  /** Per-page reasons for every page in `pagesNeedingOcr`. */
  ocrReasons: Array<PdfOcrPage>
  /** Detection confidence, 0.0 to 1.0. */
  confidence: number
}

/** One page that needs OCR, and why. */
export interface PdfOcrPage {
  /** 1-indexed page number. */
  page: number
  /**
   * Machine-readable reason codes: `scanned`, `no_text`, `vector_text`, or
   * `suspected_garbled_text`.
   */
  reasons: Array<string>
}
"#;
