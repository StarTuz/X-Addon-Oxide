# Flight Generator: Full Research, Assessment & Plan (No Coding)

## 1. Evidence from Your Environment

### 1.1 Screenshots

**Flight Gen tab**
- "Flight from Southern California to Oregon" → **Error: No suitable destination found.**
- "Flight from Riverside County to Oregon" → **Error: No suitable departure airport found.**
- Duration 245 mins visible; export buttons (FMS 11/12, LNM, SimBrief) present.

**Scenery tab**
- **Only 4 scenery packs** listed: LFAE Eu Mers Le Tréport, MRGP Guapiles, KPSP Palm Springs, aaa_Boundless_EGHE.
- **No "Global Airports" or *GLOBAL_AIRPORTS*** in the list.
- Profile: Winter. X-Plane path: `/xplane/x-plane/X-Plane12`.
- Map shows Southern California; Inspector shows heliport 3CA8 (Holy Cross Medical Center).

So in your setup, the **scenery library that the app uses for flight gen is exactly those 4 packs**. Whatever is in `scenery_packs.ini` for that install/profile is what gets loaded; there is no separate “inject Global Airports for flight gen” step.

### 1.2 Log (x-adox.log)

- Log contains **no SceneryManager or flight_gen messages**. Only init, path normalization, config root, BitNet heuristics, profiles, winit/GPU/tracing.
- **Reason:** SceneryManager (and related code) uses **`println!`**, which goes to **stdout**, not to the log file. So load results, pack count, “Global Airports discovery”, “Resources fallback”, and any parse errors are **invisible** in the log. There is no way to diagnose “why no airports?” from the current log.

---

## 2. Root Cause Analysis

### 2.1 Data source is strictly INI-driven

- Flight gen receives **`packs: &[SceneryPack]`** from the GUI. Those packs come from **one** place: **SceneryManager::load()**, which reads **`scenery_packs.ini`** and then discovers airports (and tiles) for each pack.
- **We do not auto-add** Global Airports to the INI (by design). So if the INI does not contain an entry for Global Airports (or *GLOBAL_AIRPORTS*), **that pack never exists** in `packs`, and flight gen never sees any of its airports.
- Your Scenery tab shows only 4 packs → the INI (for that install/profile) effectively has only those 4 entries (or only those 4 are active/visible). So **all airport data available to flight gen is whatever those 4 packs contain** (a few specific airports: France, Costa Rica, Palm Springs, EGHE). No UK, no Italy, no broad US coverage, no Oregon.

So the first structural issue: **flight gen’s “world” is exactly the union of pack airport lists. If Global Airports isn’t in the INI (or doesn’t load), that world is tiny and region-based queries fail.**

### 2.2 Location parsing is a growing, inconsistent patchwork

- **City/country → region** is done in two ways:
  - **RegionIndex.search()** (e.g. “Italy”, “Oregon”) → region by name/id in `regions.json`.
  - **Hardcoded aliases** in `try_as_region()`: “london”, “uk”, “socal”, “norcal”, “pnw”, “italy”, “paris”, etc.
- **Gaps:**
  - “Riverside County” is **not** in the alias list and is **not** a region name in `regions.json`. So it is parsed as **AirportName("riverside county")**. Then we need an airport whose name/ICAO contains that string; with only 4 packs we have none → **No suitable departure airport found.** So one phrase works (e.g. “Southern California”) and a more specific one (Riverside County) does not, with no principled way to extend.
  - “Oregon” works (region name in JSON). “Southern California” works (alias → US:SoCal). So parsing can succeed for both origin and destination and we still get “No suitable destination” (see next).

### 2.3 Region → airport lookup depends on packs + seeds; seeds are incomplete

- For **Region(US:SoCal)** and **Region(US:OR)**:
  - **icao_prefixes_for_region()** only has **"US"** (prefix `K`). It does **not** have **"US:SoCal"** or **"US:OR"**. So for those IDs we get `None` and rely only on **bounds** from `regions.json`.
  - **get_seed_airports_for_region()** only has **"US"** (KJFK, KLAX, KORD, KATL). It does **not** have **"US:SoCal"** or **"US:OR"**. So for those IDs we get **empty seeds**.
- So:
  - **Origin (US:SoCal):** Candidates = airports from packs that lie in SoCal bounds. Of your 4 packs, only KPSP (Palm Springs) is in Southern California. So you may get one origin.
  - **Destination (US:OR):** Candidates = airports from packs in Oregon bounds. None of the 4 packs are in Oregon → **zero destinations** → **No suitable destination found.**
  - Seeds are never used for US:SoCal or US:OR because the seed list is keyed only by country-level `"US"`, not by sub-region IDs. So the “fix” that added seeds only helps the **exact** regions we hardcoded (UK, IT, FR, US, etc.), not the same regions when requested as **sub-regions** (US:SoCal, US:OR).

So the second structural issue: **region IDs in the parser (and regions.json) are richer than the seed table and the ICAO prefix table.** Any request that uses a sub-region (US:SoCal, US:OR, etc.) gets no seeds and no prefix, so success depends entirely on pack data.

### 2.4 Why “London to Italy” could work after the “fixes”

- “London” → alias → **Region(UK)**. “Italy” → **Region(IT)**.
- Seeds have **"UK"** and **"IT"** with multiple airports. So even with **zero** pack data, flight gen falls back to seeds and returns a plan. That’s why that case “feels hard coded”: it’s the path we explicitly added (UK, IT, and a few other country-level IDs).
- As soon as the user says “Southern California to Oregon” or “Riverside County to Oregon”, we hit region IDs or parse results that **don’t** have seeds or aliases → back to “no data”.

### 2.5 Observability is missing

- **SceneryManager** uses **println!** for: INI path, pack count, discovered count, Global Airports discovery, path sync, Resources fallback, pack init with N airports, parse errors. **None of this appears in x-adox.log.**
- **Flight gen** does not log: which constraint was used (Region vs AirportName), how many candidates came from packs vs seeds, or why the list was empty. So when the user sees “No suitable departure/destination found”, there is **no trace** of: “we had Region(US:SoCal), 1 pack origin, 0 pack destinations, 0 seeds for US:SoCal/US:OR”.

So the third structural issue: **we cannot diagnose failures from the log.** The current “fixes” were done by reasoning about code paths, not by observing real runs.

---

## 3. Assessment: Why This Feels Like Bandaids

1. **No single source of truth for “where do airports come from?”**  
   We have: INI → packs → discover per pack (Global Scenery path + Resources fallback) + seed list in flight_gen. If the INI doesn’t list Global Airports, the Resources fallback never runs for “Global Airports” because that pack doesn’t exist. Seeds are a second, ad-hoc source that only applies when (a) the constraint is Region and (b) the region ID is one of the ~15 we listed. So we have two partial fixes that don’t align with how the pipeline actually runs (INI-first, pack-centric).

2. **Location handling is not systematic.**  
   We mix: 4-letter ICAO, a few country/region names from RegionIndex, and a hardcoded alias list. There is no rule like “any phrase that matches a region name or alias becomes Region(id); otherwise AirportName”. So “Riverside County” and similar will keep failing until someone adds another string to the alias list. That’s inherently brittle.

3. **Seeds and region IDs are out of sync.**  
   regions.json has many IDs (US, US:SoCal, US:OR, US:NorCal, …). Seeds and icao_prefixes only handle a subset (e.g. US, not US:SoCal/US:OR). So the same logical “place” can be requested in two ways (e.g. “California” vs “Southern California”), and one path gets seeds and one doesn’t. That’s inconsistent and hard to maintain.

4. **No visibility into why a request failed.**  
   Without logging, we can’t tell if the failure was: no pack data, wrong region ID, missing alias, empty seeds, or something else. So every “fix” is a guess.

5. **Global Airports is optional in the UI.**  
   The app never guarantees “Global Airports is in the pack list”. So flight gen is expected to work both when (a) the user has a full INI with Global Airports and (b) when they have a minimal or profile-specific INI with only a few custom packs. The current design only robustly handles (a) if Global Airports actually loads, and (b) only for the few region IDs we put in the seed list. That’s not a clear contract.

---

## 4. Plan (High Level, No Code Yet)

### 4.1 Clarify the contract for airport data

- **Option A – Pack-only (strict):**  
  “Flight gen only uses airports from scenery packs that appear in the current INI.” Then we must: (1) document that Global Airports (or equivalent) should be in the INI for region-based flights, and (2) optionally add a **detection + one-time suggestion** in the UI: “Global Airports not in scenery list; add it for better flight suggestions?”
- **Option B – Guaranteed base layer:**  
  “Flight gen always has a base airport set.” Then we need a **first-class** “default airport database” that is loaded **independently of the INI**: e.g. always try `Resources/.../apt.dat` (and optionally Global Scenery) at load time, and expose that as a dedicated “base” or “global” pack for flight gen only (not necessarily shown in the Scenery tab). So flight gen’s input = INI packs + this base layer.
- **Option C – Seeds as the base:**  
  “Seeds are the fallback for any region.” Then seeds must be **complete and consistent** with all region IDs we ever produce (including US:SoCal, US:OR, etc.), and we need a rule for “unknown” region IDs (e.g. fall back to parent region like US or to a generic “US” seed list). That implies a maintained seed dataset and a clear region hierarchy (e.g. US:SoCal → US).

Recommendation: decide explicitly between A, B, or C (or a hybrid, e.g. B + C with seeds only when base layer fails). Right now we have an implicit hybrid with no guarantee.

### 4.2 One source of truth for “place” → “region”

- Define a **single** resolution path: e.g. (1) 4-letter ICAO, (2) RegionIndex (by id, then name, then alias table derived from or consistent with regions.json), (3) then AirportName for the rest.
- Move **all** “place” strings (cities, counties, states, nicknames) into that one path: either in regions.json (with optional aliases) or in a single alias → region_id table that is **generated or validated** against regions.json. No second ad-hoc list in try_as_region() that can drift.
- Then document: “If it’s not an ICAO and not in the region/alias table, we treat it as an airport name (fuzzy match).” So “Riverside County” would either get an alias (e.g. → US:SoCal or a new region) or be clearly “airport name only”.

### 4.3 Align seeds and ICAO prefixes with region IDs

- Either:
  - **Reduce:** Only produce country-level region IDs from the parser (e.g. “Southern California” → US, “Oregon” → US), so existing US seeds and prefix “K” apply; or
  - **Expand:** For every region ID that can be produced (including US:SoCal, US:OR, etc.), add seeds and ICAO prefix rules (or a rule: “if no prefix for ID, use parent ID’s prefix”), so that seeds and prefixes are never empty for a valid region ID.
- Prefer one rule set (e.g. “always fall back to parent region for seeds/prefix”) so we don’t maintain N copies of the same list.

### 4.4 Observability

- **Replace or supplement println! in SceneryManager** with **log::info!** (or equivalent) so that: INI path, number of packs read, Global Airports discovery result, Resources fallback use, and per-pack airport counts (at least for Global Airports) appear in **x-adox.log**.
- **Add minimal flight_gen logging** (e.g. at debug level): parsed origin/destination constraint types and IDs, number of candidates from packs, whether seeds were used, and which step failed when returning “No suitable departure/destination found”. So a support or dev can open the log and see the exact path for a failing request.

### 4.5 User-facing clarity (optional but recommended)

- In Flight Gen UI: if the last error was “no suitable departure/destination” and the app knows that Global Airports is not in the pack list (or has zero airports), show a short hint: “Tip: Add Global Airports to your Scenery Library for more flight options,” with a link or button to the Scenery tab.
- Optionally, in Scenery tab, if the INI has no Global Airports entry, show a non-blocking notice: “Global Airports not in list; flight suggestions may be limited.”

---

## 5. Summary

- **Why you see the errors:**  
  (1) Only 4 packs in the list and no Global Airports → almost no airport data.  
  (2) “Southern California to Oregon” uses region IDs US:SoCal and US:OR, which have no seeds and no ICAO prefix in code, so only pack data is used → no Oregon airports.  
  (3) “Riverside County” is not a recognized region or alias → treated as airport name → no match in 4 packs.

- **Why it feels like bandaids:**  
  The pipeline is INI → packs → discover; then flight gen uses packs + a small, hardcoded seed list keyed by a subset of region IDs. Fixes so far added: Resources fallback (only when a Global Airports pack exists), and seeds for a few country-level IDs. There is no single design for “where airports come from,” no systematic “place → region,” and no logging, so behavior is opaque and each fix is local.

- **What’s needed before more code:**  
  Decide the **airport data contract** (pack-only vs guaranteed base layer vs expanded seeds), introduce **one place** for “place” → region (and align seeds/prefixes with that), and add **logging** so failures are diagnosable. Then implement against that plan instead of adding more one-off branches.
