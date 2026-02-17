#!/usr/bin/env python3
"""
Generate flight_context_bundle.json from icao_to_wikipedia.json by fetching
Wikipedia summary API for each ICAO. Run from repo root. Requires network.

Usage:
  python3 scripts/generate_flight_context_bundle.py

Output: crates/x-adox-core/data/flight_context_bundle.json

To expand the bundle: add more ICAO → Wikipedia title entries to
crates/x-adox-core/data/icao_to_wikipedia.json, then re-run this script.
"""

import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "crates", "x-adox-core", "data")
MAP_FILE = os.path.join(DATA_DIR, "icao_to_wikipedia.json")
OUT_FILE = os.path.join(DATA_DIR, "flight_context_bundle.json")
WIKI_SUMMARY = "https://en.wikipedia.org/api/rest_v1/page/summary"


def fetch_summary(title: str) -> str | None:
    url = f"{WIKI_SUMMARY}/{urllib.parse.quote(title, safe='')}"
    req = urllib.request.Request(url, headers={"User-Agent": "X-Addon-Oxide/1.0 (bundle generator)"})
    try:
        with urllib.request.urlopen(req, timeout=15) as r:
            data = json.loads(r.read().decode())
            return (data.get("extract") or "").strip() or None
    except (urllib.error.URLError, json.JSONDecodeError, KeyError) as e:
        print(f"  skip {title}: {e}", file=sys.stderr)
        return None


def main() -> int:
    if not os.path.isfile(MAP_FILE):
        print(f"Missing {MAP_FILE}", file=sys.stderr)
        return 1
    with open(MAP_FILE, encoding="utf-8") as f:
        icao_to_title = json.load(f)
    bundle = {}
    for icao, title in sorted(icao_to_title.items()):
        print(f"Fetching {icao} ({title})...", file=sys.stderr)
        snippet = fetch_summary(title)
        if snippet:
            bundle[icao] = {"snippet": snippet, "points_nearby": []}
    with open(OUT_FILE, "w", encoding="utf-8") as f:
        json.dump(bundle, f, ensure_ascii=False, indent=None)
    print(f"Wrote {len(bundle)} airports to {OUT_FILE}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
