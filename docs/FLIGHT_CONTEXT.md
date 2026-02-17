# Flight Context: Landmarks & Airport History

Developer reference for the "History & context" panel in the Flight Generator tab.

## Architecture

```
User clicks "Fetch context" (or auto-fetch after generation)
    │
    ▼
main.rs: Message::FetchContext / auto-fetch block
    │  spawns blocking task
    ▼
flight_gen_gui.rs: load_or_fetch_flight_context_blocking()
    │
    ├─ fetch_pois_near_from_wikipedia()   Wikipedia geosearch API
    ├─ fetch_pois_near_from_wikidata()    Wikidata SPARQL query
    ├─ merge_pois_dedupe_by_title()       Merge + deduplicate
    ├─ enrich_pois_with_extracts()        Fetch Wikipedia summaries
    └─ fetch_airport_context_from_wikipedia()  Airport history snippet
    │
    ▼
flight_gen.rs: load_flight_context_with_bundled()
    │  Merges: cache → bundled → overlay → dynamic POIs
    │  Filters to 10 nm radius via build_airport_context()
    ▼
FlightContext { origin: AirportContext, destination: AirportContext }
    │
    ▼
flight_gen_gui.rs: apply_fetched_context() → updates UI
```

## Data Sources (in merge priority order)

| Source | File / Location | Coverage | Notes |
|--------|----------------|----------|-------|
| **Per-ICAO cache** | `flight_context_cache/{ICAO}.json` | Any previously fetched | Written by `save_airport_context_to_cache()`. History snippet + POIs. |
| **Bundled** | `x-adox-core/data/flight_context_bundle.json` | 63 airports | Compiled into binary. Has snippets but `points_nearby: []` for all. |
| **POI overlay** | `x-adox-core/data/flight_context_pois_overlay.json` | EGLL, LIRF only | Curated landmarks. Compiled into binary. |
| **User config** | `~/.config/x-adox/X-Addon-Oxide/flight_context.json` | User-added | Optional manual overrides. |
| **Wikipedia geosearch** | API: `en.wikipedia.org/w/api.php` | Any lat/lon | 10 km radius, 20 results max. Enhanced mode only. |
| **Wikidata SPARQL** | API: `query.wikidata.org/sparql` | Any lat/lon | 20 km radius, 30 results. Stadiums, museums, landmarks, piers, tourist attractions. |

## Cache Layout

All under `~/.config/x-adox/X-Addon-Oxide/flight_context_cache/`:

```
flight_context_cache/
├── EGLL.json              # Per-ICAO: { snippet, points_nearby[] }
├── LFPG.json
├── pois_near/             # Wikipedia geosearch cache
│   └── 51.470_-0.454.json # Key: lat/lon rounded to 3 decimals
├── pois_near_wikidata/    # Wikidata SPARQL cache
│   └── 51.470_-0.454.json
└── poi_extract/           # Wikipedia page summary cache
    └── Tower_of_London.json
```

**TTL**: 7 days for POI and extract caches (`POIS_NEAR_CACHE_TTL_SECS`, `POI_EXTRACT_CACHE_TTL_SECS`).

## Key Functions

### `flight_gen_gui.rs` (GUI crate)

| Function | Line | Purpose |
|----------|------|---------|
| `load_flight_context_for_plan()` | ~563 | Non-enhanced path: bundled + cache only, no API calls. Passes `None` for dynamic POIs. |
| `load_or_fetch_flight_context_blocking()` | ~1007 | Full enhanced path: fetches Wikipedia + Wikidata POIs, enriches with extracts, fetches airport snippets. |
| `fetch_pois_near_from_wikipedia()` | ~722 | Wikipedia geosearch API. Returns `Option<Vec<PoiFile>>`. |
| `fetch_pois_near_from_wikidata()` | ~814 | Wikidata SPARQL query for typed landmarks. Returns `Option<Vec<PoiFile>>`. |
| `merge_pois_dedupe_by_title()` | ~932 | Merges Wikipedia + Wikidata results, deduplicates by title (Wikipedia preferred). |
| `enrich_pois_with_extracts()` | ~702 | Fetches Wikipedia page summaries for up to N POIs. Replaces title-only snippets with descriptions. |
| `fetch_airport_context_from_wikipedia()` | ~979 | Fetches airport history via ICAO→Wikipedia title map. Saves to per-ICAO cache. |
| `apply_fetched_context()` | ~293 | Applies `FlightContext` result to GUI state. Sets status message based on landmark availability. |

### `flight_gen.rs` (core crate)

| Function | Line | Purpose |
|----------|------|---------|
| `load_flight_context_with_bundled()` | ~224 | Merges all sources: cache → bundled → overlay → dynamic. Calls `build_airport_context()`. |
| `build_airport_context()` | ~314 | Chains all POI sources, filters to 10 nm, excludes airport's own Wikipedia article. |
| `airport_coords_for_poi_fetch()` | ~191 | Returns (lat, lon) from airport struct or hardcoded fallback for known ICAOs. |
| `get_bundled_flight_context()` | ~132 | Parses embedded `flight_context_bundle.json`. |
| `get_poi_overlay()` | ~141 | Parses embedded `flight_context_pois_overlay.json`. |
| `get_icao_to_wikipedia()` | ~172 | Parses embedded `icao_to_wikipedia.csv` → ICAO→title map (used for airport snippet fetch). |

## Fetch vs Auto-Fetch

- **"Fetch context" button** (`Message::FetchContext`): Always uses enhanced mode (Wikipedia + Wikidata). Independent of the `flight_context_fetch_enabled` setting.
- **Auto-fetch after generation** (in the `_` arm after plan generation): Only triggers when `flight_context_fetch_enabled` is `true` AND the generated plan has empty context (`plan_context_is_empty()`).

## Debugging

### Log lines to look for

```
[flight_context] Origin EGLL: Wikipedia and Wikidata POI fetch both empty
[flight_context] Destination LFPG: Wikipedia and Wikidata POI fetch both empty
```

These indicate the API calls returned no results (network issue, rate limit, or genuinely no nearby POIs).

### Cache inspection

```bash
# Check if POIs were cached for a location
ls ~/.config/x-adox/X-Addon-Oxide/flight_context_cache/pois_near/
ls ~/.config/x-adox/X-Addon-Oxide/flight_context_cache/pois_near_wikidata/

# View cached POIs for a specific coordinate bucket
cat ~/.config/x-adox/X-Addon-Oxide/flight_context_cache/pois_near/51.470_-0.454.json

# Check airport history cache
cat ~/.config/x-adox/X-Addon-Oxide/flight_context_cache/EGLL.json

# Force re-fetch by deleting cache (will re-fetch on next click)
rm ~/.config/x-adox/X-Addon-Oxide/flight_context_cache/pois_near/*.json
```

### Common issues

1. **"No landmarks in range"**: Cache may have stale empty results. Delete the relevant `pois_near/` and `pois_near_wikidata/` cache files and re-fetch.
2. **Wikipedia rate limiting**: The agent uses `User-Agent: X-Addon-Oxide/1.0 (flight context)`. Wikimedia allows ~200 req/s for identified agents. Unlikely to hit limits in normal use.
3. **Wikidata SPARQL timeout**: Complex queries for areas with many entities can time out. The 30-second `POI_FETCH_TIMEOUT_SECS` should handle most cases.
4. **Missing airport coordinates**: `airport_coords_for_poi_fetch()` has hardcoded fallbacks for a few known ICAOs. For unknown airports without lat/lon, POI fetch is skipped entirely.

## Known Limitations

- **Bundled data has empty POIs**: All 63 airports in `flight_context_bundle.json` have `"points_nearby": []`. POIs only come from the overlay (2 airports), Wikipedia, or Wikidata.
- **Overlay coverage**: Only EGLL and LIRF have curated POI overlays.
- **Geosearch radius**: Wikipedia API caps at 10 km (~5.4 nm). Wikidata SPARQL uses 20 km. Both filtered to 10 nm by `build_airport_context()`.
- **Extract enrichment limit**: Only the first 8 POIs get Wikipedia summary enrichment (to limit API calls).
- **Snippet length**: POI descriptions are truncated to 280 characters (`POI_SNIPPET_MAX_LEN`).
- **English only**: All Wikipedia/Wikidata queries target English-language content.
