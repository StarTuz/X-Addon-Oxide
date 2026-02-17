#!/usr/bin/env python3
"""
Generate icao_to_wikipedia.json from OurAirports airports.csv (wikipedia_link column).
Scales to thousands of airports without manual curation. Run from repo root. Requires network.

Usage:
  python3 scripts/generate_icao_to_wikipedia_from_ourairports.py

Output: crates/x-adox-core/data/icao_to_wikipedia.json and icao_to_wikipedia.csv (ident,title for runtime parsing).

OurAirports CSV: https://davidmegginson.github.io/ourairports-data/airports.csv
Columns used: ident (ICAO/FAA code), wikipedia_link (URL like https://en.wikipedia.org/wiki/Heathrow_Airport).
"""

import csv
import json
import os
import sys
import urllib.request
import urllib.parse

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "crates", "x-adox-core", "data")
OUT_JSON = os.path.join(DATA_DIR, "icao_to_wikipedia.json")
OUT_CSV = os.path.join(DATA_DIR, "icao_to_wikipedia.csv")
AIRPORTS_CSV_URL = "https://davidmegginson.github.io/ourairports-data/airports.csv"


def extract_title_from_wikipedia_url(url: str) -> str | None:
    if not url or "wikipedia.org/wiki/" not in url:
        return None
    try:
        # e.g. https://en.wikipedia.org/wiki/Heathrow_Airport or .../wiki/Paris
        path = urllib.parse.urlparse(url.strip()).path
        if path.startswith("/wiki/"):
            title = path[6:]  # strip /wiki/
            title = urllib.parse.unquote(title).replace(" ", "_")
            return title if title else None
    except Exception:
        pass
    return None


def main() -> int:
    print("Downloading OurAirports airports.csv...", file=sys.stderr)
    try:
        with urllib.request.urlopen(urllib.request.Request(
            AIRPORTS_CSV_URL,
            headers={"User-Agent": "X-Addon-Oxide/1.0 (bundle generator)"}
        ), timeout=60) as r:
            raw = r.read().decode("utf-8", errors="replace")
    except Exception as e:
        print(f"Download failed: {e}", file=sys.stderr)
        return 1
    icao_to_title = {}
    reader = csv.DictReader(raw.splitlines())
    if "ident" not in reader.fieldnames or "wikipedia_link" not in reader.fieldnames:
        print("CSV missing ident or wikipedia_link column", file=sys.stderr)
        return 1
    for row in reader:
        ident = (row.get("ident") or "").strip()
        link = (row.get("wikipedia_link") or "").strip()
        if not ident or not link:
            continue
        title = extract_title_from_wikipedia_url(link)
        if title:
            icao_to_title[ident] = title
    with open(OUT_JSON, "w", encoding="utf-8") as f:
        json.dump(icao_to_title, f, ensure_ascii=False, indent=None)
    with open(OUT_CSV, "w", encoding="utf-8", newline="") as f:
        w = csv.writer(f)
        w.writerow(["ident", "title"])
        for ident, title in sorted(icao_to_title.items()):
            w.writerow([ident, title])
    print(f"Wrote {len(icao_to_title)} ICAO → Wikipedia to {OUT_JSON} and {OUT_CSV}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
