# Flight Generator: Airport Data Sources & Robustness

## Problem

"Flight from London to Italy" (and similar region-based requests) can fail with **"No suitable departure airport found"** even when the user has:

- Global Airports (or `*GLOBAL_AIRPORTS*`) present and enabled in the Scenery tab
- X-Plane path set to the real X-Plane 12 root
- Refreshed scenery so the app has loaded

So the failure is not just parsing ("London" → region); it is that **no pack in the app’s list ends up with any airports** for the requested region.

## Root Causes (Research)

### 1. Where does X-Plane store default airport data?

- **Assumption in code:** `Global Scenery/Global Airports/Earth nav data/apt.dat`  
  Used in `lib.rs` (`get_default_apt_dat_path`) and in SceneryManager (we discover that folder and run `discover_airports_in_pack` on it).

- **Alternative location (X-Plane 12):**  
  Some installs and docs use **`Resources/default scenery/default apt dat/Earth nav data/apt.dat`** for the main airport database. The e2e tests in this repo create that path and parse it directly; SceneryManager never loads from there.

- **Implication:** If XP12 ships only (or primarily) the default apt.dat under **Resources**, then scanning only **Global Scenery/Global Airports** can yield **zero airports**, so flight gen has nothing to work with.

### 2. Cache and path sync

- Cache is keyed by `pack.path` (PathBuf). After INI read, the Global Airports pack is reconciled so `path` becomes `xplane_root/Global Scenery/Global Airports`.
- If the cache was ever populated with a different path or with empty airports (e.g. wrong path in the past), a cache hit can keep returning empty airports until mtime changes or cache is invalidated.

### 3. Structure of Global Scenery/Global Airports

- We only look for **one** `Earth nav data` and **one** `apt.dat` per “root” (and we have a fallback for apt.dat in the pack root). If XP12 uses a **tile-based** layout (e.g. `+50-010/Earth nav data/apt.dat`) with no top-level `Earth nav data`, `find_pack_roots` would still find subdirs that look like scenery roots and we’d parse those; that’s probably OK. The bigger risk is the **main** apt.dat living only under Resources.

### 4. What we do **not** want to depend on

- **OpenStreetMap / external API:** Would require network, rate limits, and goes against the app’s offline design. Not chosen for core flow.
- **Assuming a single canonical path:** Relying only on one of the two possible default apt.dat locations is fragile across XP versions and installs.

## Options for Robustness

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **A. Resources fallback** | When the Global Airports pack has zero airports, also try loading `Resources/default scenery/default apt dat/Earth nav data/apt.dat` and merge. | Uses only existing XP files; no new data; fixes XP12 if they use Resources. | Slightly more code; two paths to maintain. |
| **B. Embedded seed airports** | Ship a small list of major airports (ICAO, name, lat, lon, region) and use it in flight_gen when packs yield no candidates for the requested region. | Works even with broken/missing scenery or cache; no dependency on XP file layout. | Small list; not a full DB; only used as fallback. |
| **C. Recursive apt.dat discovery** | Walk the whole pack (and optionally Resources) for any `apt.dat` and parse all. | Handles nested or non-standard layouts. | Heavier I/O; need to avoid double-counting. |
| **D. OSM / external API** | Resolve “London” to bbox or ICAOs via network. | Rich data. | Network, privacy, complexity; not aligned with offline-first. |

**Chosen:** **A + B**  
- **A** fixes the common case where the default apt.dat is under Resources or where Global Scenery/Global Airports has no apt.dat.  
- **B** guarantees that region-based requests (e.g. “London to Italy”) always get at least a sensible suggestion even if both XP paths fail or cache is wrong.

## Implementation Summary

1. **Resources fallback (scenery load)**  
   When processing the Global Airports pack, if `discover_airports_in_pack(pack.path)` returns empty, derive `xplane_root` from `pack.path` and try loading `xplane_root/Resources/default scenery/default apt dat/Earth nav data/apt.dat`. If that file exists and parses, use (or merge with) those airports for that pack.

2. **Seed airports (flight_gen)**  
   Add a small embedded list of airports (e.g. UK: EGLL, EGKK, EGCC, …; IT: LIRF, LIML, …; FR, DE, US, …) with id, name, lat, lon, and region. In `generate_flight`, when building `candidate_origins` or `candidate_dests` from packs, if the result is empty for a **Region** constraint, extend the list with seed airports for that region (and optionally filter by existing packs to prefer pack data). Then pick origin/destination from the combined list so “London to Italy” always returns a valid plan.

No OpenStreetMap or external API is required; everything stays offline and robust against missing or misconfigured XP scenery.
