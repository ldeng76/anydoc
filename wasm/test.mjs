// Smoke test: the wasm bindings load in Node and every entry point
// round-trips a fixture. Build first: wasm-pack build wasm --release --target web
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'

import {
  initSync,
  formatFromBytes,
  formatFromExtension,
  formatFromPath,
  inspectPdf,
  toDocument,
  toMarkdownBytes,
} from './pkg/anydoc_wasm.js'

const fixture = (name) => fileURLToPath(new URL(`../tests/fixtures/${name}`, import.meta.url))

initSync({ module: await readFile(fileURLToPath(new URL('./pkg/anydoc_wasm_bg.wasm', import.meta.url))) })

const OUTLINE = await readFile(fixture('docx/handmade-outline.docx'))
const RICH = await readFile(fixture('docx/handmade-rich.docx'))
const CSV = await readFile(fixture('csv/sheet.csv'))
const PDF = await readFile(fixture('pdf/text.pdf'))
const TEXT_PDF = await readFile(fixture('pdf/text-only.pdf'))
const SCANNED_PDF = await readFile(fixture('pdf/scanned.pdf'))
const ENCRYPTED = await readFile(fixture('malformed/encrypted--errors.odt'))

test('toMarkdownBytes converts in memory', () => {
  const markdown = toMarkdownBytes(RICH, 'docx')
  assert.match(markdown, /\| Quarter \| Widgets \|/)
})

test('toMarkdownBytes detects the format when none is named', () => {
  assert.match(toMarkdownBytes(RICH), /\| Quarter \| Widgets \|/)
  // CSV carries no signature, so it has to be named.
  assert.throws(() => toMarkdownBytes(CSV), /unrecognized file content/)
  assert.match(toMarkdownBytes(CSV, 'csv'), /\| --- \|/)
})

test('pdf converts to Markdown but has no document model', () => {
  assert.ok(toMarkdownBytes(PDF).length > 0)
  assert.throws(() => toDocument(PDF), /pdf/i)
})

test('inspectPdf reports whether OCR is needed', () => {
  const text = inspectPdf(TEXT_PDF)
  assert.equal(text.needsOcr, false)
  assert.equal(text.pdfType, 'textBased')
  assert.deepEqual(text.pagesNeedingOcr, [])
  assert.deepEqual(text.ocrReasons, [])
  assert.ok(text.confidence >= 0 && text.confidence <= 1)

  const scanned = inspectPdf(SCANNED_PDF)
  assert.equal(scanned.needsOcr, true)
  assert.equal(scanned.pdfType, 'scanned')
  assert.equal(scanned.pageCount, 1)
  assert.deepEqual(scanned.pagesNeedingOcr, [1])
  assert.equal(scanned.ocrReasons[0].page, 1)
  assert.ok(scanned.ocrReasons[0].reasons.includes('scanned'))
})

test('inspectPdf returns null for non-PDFs and throws for corrupt PDFs', () => {
  assert.equal(inspectPdf(new TextEncoder().encode('not a pdf')), null)
  assert.equal(inspectPdf(CSV), null)
  assert.throws(
    () => inspectPdf(new TextEncoder().encode('%PDF-1.7\nthis is not a valid pdf body at all')),
    (error) => {
      assert.ok(error instanceof Error)
      assert.equal(error.code, 'malformed')
      return true
    },
  )
})

test('toDocument exposes the document model', () => {
  const document = toDocument(OUTLINE, 'docx')
  const heading = document.blocks.find((block) => block.kind === 'heading')
  assert.ok(heading.level >= 1 && heading.level <= 6)
  assert.equal(typeof heading.content[0].text, 'string')
  assert.equal(heading.content[0].kind, 'text')
  assert.equal(typeof heading.content[0].style.bold, 'boolean')
})

test('toDocument carries embedded assets as Uint8Arrays', () => {
  const document = toDocument(RICH, 'docx')
  const image = document.assets.find((asset) => asset.mediaType === 'image/png')
  assert.ok(image.data instanceof Uint8Array)
  assert.ok(image.data.length > 0)
  assert.equal(image.id, document.assets.indexOf(image))
})

test('format detection reads content, extension, and path', () => {
  assert.equal(formatFromBytes(RICH), 'docx')
  // CSV carries no signature: only the extension names it.
  assert.equal(formatFromBytes(CSV), undefined)
  assert.equal(formatFromExtension('.pptm'), 'pptx')
  assert.equal(formatFromExtension('xls'), 'xlsx')
  assert.equal(formatFromPath('/tmp/report.odt'), 'odt')
  assert.equal(formatFromPath('/tmp/report.unknown'), undefined)
})

// `code` is what callers branch on, so every kind of failure is pinned here.
test('conversion errors throw a coded Error', () => {
  const throws = (call, code, message) =>
    assert.throws(call, (error) => {
      assert.ok(error instanceof Error)
      assert.equal(error.code, code)
      assert.match(error.message, message)
      return true
    })

  throws(() => toMarkdownBytes(new TextEncoder().encode('not a document'), 'docx'), 'malformed', /malformed/)
  throws(() => toMarkdownBytes(CSV), 'unsupported', /unrecognized file content/)
  throws(() => toMarkdownBytes(ENCRYPTED, 'odt'), 'encrypted', /encrypted/)
  throws(() => toDocument(ENCRYPTED, 'odt'), 'encrypted', /encrypted/)
})
