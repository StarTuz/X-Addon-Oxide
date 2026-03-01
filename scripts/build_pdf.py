#!/usr/bin/env python3
"""
Build the X-Addon-Oxide User Manual PDF from HANDBOOK.md.

Usage:
    python3 scripts/build_pdf.py [--output path/to/output.pdf]

Requirements:
    pip install markdown
    chromium (or google-chrome) installed on PATH
"""

import argparse
import markdown
import re
import os
import shutil
import subprocess
import sys
import tempfile

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HANDBOOK_MD = os.path.join(REPO_ROOT, "HANDBOOK.md")
DEFAULT_OUTPUT = os.path.join(REPO_ROOT, "X-Addon-Oxide-User-Manual.pdf")

PROFESSIONAL_CSS = """
  @page {
    size: A4;
    margin: 20mm 18mm 22mm 18mm;
  }
  * { box-sizing: border-box; }
  body {
    font-family: "Helvetica Neue", Helvetica, Arial, sans-serif;
    font-size: 10.5pt;
    line-height: 1.65;
    color: #1a1a2e;
    max-width: 100%;
    margin: 0;
    padding: 0;
  }
  h1 { font-size: 22pt; color: #1a237e; border-bottom: 3px solid #1565c0; padding-bottom: 8px; margin-top: 32px; margin-bottom: 12px; page-break-after: avoid; }
  h2 { font-size: 15pt; color: #283593; border-bottom: 1.5px solid #bbdefb; padding-bottom: 5px; margin-top: 24px; margin-bottom: 10px; page-break-after: avoid; }
  h3 { font-size: 12pt; color: #37474f; margin-top: 18px; margin-bottom: 8px; page-break-after: avoid; }
  h4 { font-size: 10.5pt; color: #455a64; margin-top: 14px; margin-bottom: 6px; page-break-after: avoid; }
  p { margin: 0 0 10px 0; orphans: 3; widows: 3; }
  a { color: #1565c0; text-decoration: none; }
  ul, ol { margin: 6px 0 10px 0; padding-left: 22px; }
  li { margin-bottom: 4px; }
  code {
    background-color: #f1f3f4;
    padding: 2px 6px;
    border-radius: 3px;
    font-family: "Courier New", Courier, monospace;
    font-size: 9pt;
    color: #c62828;
    border: 1px solid #e0e0e0;
  }
  pre {
    background-color: #263238;
    color: #eceff1;
    padding: 14px 16px;
    border-radius: 6px;
    overflow-x: auto;
    font-family: "Courier New", Courier, monospace;
    font-size: 8.5pt;
    line-height: 1.5;
    margin: 12px 0;
    page-break-inside: avoid;
  }
  pre code {
    background: none;
    padding: 0;
    border: none;
    color: #eceff1;
    font-size: 8.5pt;
  }
  blockquote {
    border-left: 4px solid #1565c0;
    margin: 12px 0;
    padding: 10px 16px;
    background: #e3f2fd;
    color: #1a237e;
    border-radius: 0 4px 4px 0;
    font-style: italic;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    margin: 14px 0;
    font-size: 9.5pt;
    page-break-inside: avoid;
  }
  th {
    background: #1565c0;
    color: white;
    padding: 9px 12px;
    text-align: left;
    font-weight: 600;
    font-size: 9pt;
  }
  td {
    padding: 8px 12px;
    border: 1px solid #e0e0e0;
    vertical-align: top;
  }
  tr:nth-child(even) td { background-color: #f8f9ff; }
  tr:nth-child(odd) td { background-color: #ffffff; }
  img {
    max-width: 100%;
    height: auto;
    border: 1px solid #e0e0e0;
    border-radius: 6px;
    box-shadow: 0 2px 8px rgba(0,0,0,0.12);
    margin: 14px 0;
    display: block;
  }
  .cover-page {
    text-align: center;
    padding: 60px 40px 80px;
    page-break-after: always;
    background: linear-gradient(135deg, #0d1b2a 0%, #1a237e 50%, #283593 100%);
    color: white;
    min-height: 260mm;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }
  .cover-page img { border: none; box-shadow: none; margin: 0 auto 24px; }
  .cover-title { font-size: 40pt; font-weight: 800; color: #ffffff; margin: 16px 0 6px; letter-spacing: -0.5px; }
  .cover-subtitle { font-size: 18pt; color: #90caf9; margin: 6px 0 40px; font-weight: 300; }
  .cover-version { font-size: 13pt; color: #bbdefb; margin: 6px 0; }
  .page-break { page-break-after: always; height: 0; }
  .note {
    background: #fffde7;
    border: 1px solid #f9a825;
    border-left: 5px solid #f9a825;
    padding: 12px 16px;
    border-radius: 0 6px 6px 0;
    margin: 14px 0;
    color: #4a3900;
    font-size: 9.5pt;
  }
  .tip {
    background: #e8f5e9;
    border: 1px solid #43a047;
    border-left: 5px solid #43a047;
    padding: 12px 16px;
    border-radius: 0 6px 6px 0;
    margin: 14px 0;
    color: #1b5e20;
    font-size: 9.5pt;
  }
  .warning {
    background: #fce4ec;
    border: 1px solid #e53935;
    border-left: 5px solid #e53935;
    padding: 12px 16px;
    border-radius: 0 6px 6px 0;
    margin: 14px 0;
    color: #7f0000;
    font-size: 9.5pt;
  }
  hr { border: none; border-top: 2px solid #e3e8f0; margin: 28px 0; }
"""


def find_chromium() -> str:
    for candidate in ("chromium", "chromium-browser", "google-chrome", "google-chrome-stable"):
        path = shutil.which(candidate)
        if path:
            return path
    raise RuntimeError(
        "No Chromium/Chrome binary found. Install chromium or google-chrome."
    )


def build_pdf(output_path: str) -> None:
    print(f"Reading {HANDBOOK_MD}...")
    with open(HANDBOOK_MD, "r") as f:
        content = f.read()

    # Strip YAML front matter
    content = re.sub(r"^---.*?---\s*", "", content, flags=re.DOTALL)

    # Remove embedded <style> block (we supply our own)
    content = re.sub(r"<style>.*?</style>\s*", "", content, flags=re.DOTALL)

    # Make image paths absolute using file:// URIs
    content = content.replace(
        'src="pictures/', f'src="file://{REPO_ROOT}/pictures/'
    )
    content = content.replace(
        'src="assets/', f'src="file://{REPO_ROOT}/assets/'
    )

    # Convert markdown → HTML
    print("Converting Markdown to HTML...")
    md = markdown.Markdown(
        extensions=["tables", "fenced_code", "nl2br", "attr_list"]
    )
    body_html = md.convert(content)

    html = f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>X-Addon-Oxide User Manual v2.4.4</title>
  <style>
{PROFESSIONAL_CSS}
  </style>
</head>
<body>
{body_html}
</body>
</html>"""

    # Write temp HTML
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".html", delete=False, prefix="xao_handbook_"
    ) as tmp:
        tmp.write(html)
        tmp_path = tmp.name

    print(f"HTML written ({len(html):,} bytes). Launching Chromium...")

    chromium = find_chromium()
    cmd = [
        chromium,
        "--headless=new",
        "--no-sandbox",
        "--disable-gpu",
        "--disable-software-rasterizer",
        "--run-all-compositor-stages-before-draw",
        "--virtual-time-budget=10000",
        f"--print-to-pdf={os.path.abspath(output_path)}",
        "--print-to-pdf-no-header",
        "--no-pdf-header-footer",
        f"file://{tmp_path}",
    ]

    result = subprocess.run(cmd, capture_output=True, text=True)
    os.unlink(tmp_path)

    if result.returncode != 0:
        print("Chromium stderr:", result.stderr, file=sys.stderr)
        sys.exit(1)

    size_mb = os.path.getsize(output_path) / 1_048_576
    print(f"PDF written: {output_path} ({size_mb:.1f} MB)")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Build X-Addon-Oxide User Manual PDF")
    parser.add_argument(
        "--output", default=DEFAULT_OUTPUT, help="Output PDF path"
    )
    args = parser.parse_args()
    build_pdf(args.output)
