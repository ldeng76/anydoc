#!/usr/bin/env python3
"""Recursively scan a directory for PDF files and write each one's OCR
classification to a CSV file.

Uses the `anydoc` Python package (pip install firecrawl-anydoc) and its
detection-only `inspect_pdf` entry point, so no text extraction runs.

Usage:
    python scripts/pdf_types.py <directory> [-o out.csv]

The CSV is written UTF-8 with a BOM so Excel opens it correctly. Every PDF
gets one row: relative path, pdf_type, needs_ocr, page_count,
pages_needing_ocr (1-indexed, semicolon-separated), confidence, and the
error message when inspection failed (e.g. a corrupt PDF).
"""

import argparse
import csv
import sys
from pathlib import Path

try:
    import anydoc
except ImportError:
    print("anydoc is not installed; run: pip install firecrawl-anydoc", file=sys.stderr)
    sys.exit(2)


COLUMNS = [
    "path",
    "pdf_type",
    "needs_ocr",
    "page_count",
    "pages_needing_ocr",
    "confidence",
    "error",
]


def inspect(path: Path) -> dict[str, str]:
    """One CSV row for a PDF: the inspection, or an error row."""
    row = dict.fromkeys(COLUMNS, "")
    row["path"] = path.as_posix()
    try:
        inspection = anydoc.inspect_pdf(path)
        if inspection is None:
            row["error"] = "not a PDF"
            return row
        row["pdf_type"] = inspection.pdf_type
        row["needs_ocr"] = "true" if inspection.needs_ocr else "false"
        row["page_count"] = str(inspection.page_count)
        row["pages_needing_ocr"] = ";".join(str(page) for page in inspection.pages_needing_ocr)
        row["confidence"] = f"{inspection.confidence:.3f}"
    except Exception as error:  # noqa: BLE001 - one bad file must not stop the scan
        row["error"] = f"{type(error).__name__}: {error}"
    return row


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Scan a directory tree for PDFs and report each pdf_type to a CSV file."
    )
    parser.add_argument("directory", type=Path, help="root directory to scan recursively")
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        default=Path("pdf_types.csv"),
        help="output CSV path (default: pdf_types.csv)",
    )
    args = parser.parse_args()

    if not args.directory.is_dir():
        parser.error(f"not a directory: {args.directory}")

    pdfs = sorted(
        path for path in args.directory.rglob("*") if path.is_file() and path.suffix.lower() == ".pdf"
    )
    rows = [inspect(path) for path in pdfs]
    failed = sum(1 for row in rows if row["error"])

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="", encoding="utf-8-sig") as handle:
        writer = csv.DictWriter(handle, fieldnames=COLUMNS)
        writer.writeheader()
        writer.writerows(rows)

    print(f"scanned {len(rows)} PDF(s) under {args.directory}; {failed} failed")
    print(f"wrote {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
