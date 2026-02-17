# BitNet and Learning: Research, Assessment, and Opportunity

## 1. What “BitNet” Is Here vs in the World

### In this repo (x-adox-bitnet)

- **Name**: “BitNet” is the crate that holds the **heuristics engine** for scenery scoring and aircraft classification. The name is evocative; the implementation is **rule-based**, not a neural net.
- **What it does**:
  - **Scenery**: Keyword/regex rules → priority score (0–100). **User overrides** (pins / “sticky sort”) are stored in `heuristics.json` as `overrides: BTreeMap<pack_name, score>` and applied first in `predict()`.
  - **Aircraft**: Regex/keyword rules → tags (GA, Jet, Heavy, …). **User overrides** are stored as `aircraft_overrides: BTreeMap<name, Vec<tag>>` and applied first in `predict_aircraft_tags()`.
  - **Flight gen**: The crate also exposes `flight_prompt` (parse “Flight from X to Y”) and `geo::RegionIndex` (regions.json). **Flight generation itself lives in x-adox-core** and **now** uses flight preferences from `heuristics.json` when provided (see 3.1).

So: BitNet implements a **“user correction → persist → reuse”** pattern for scenery, aircraft, and flight gen. Flight gen is now included in that pattern (since 3.1).

### In AI research (“BitNet” the model)

- **BitNet** in the literature (e.g. Microsoft, JMLR 2024) refers to **1-bit (or ternary) Transformer LLMs**: efficient, trainable language models with very low precision weights. They are about **learnable** language understanding and generation.
- **Relevance**: Using a real BitNet-style LLM (or any small LLM) would be a **separate, large** integration (model choice, inference runtime, safety, offline story). It is **not** required to make the app “learn” in a useful sense for flight gen.

**Conclusion**: We are not “missing” the research BitNet unless we explicitly decide to add an LLM. We **are** underusing our own BitNet’s **persistence and override pattern** for flight gen.

---

## 2. Gap: Flight Gen Does Not Use BitNet’s Learning Pattern

| Feature              | Scenery                    | Aircraft                     | Flight gen (since 3.1)     |
|----------------------|----------------------------|------------------------------|--------------------------|
| Rules                | BitNet rules + regex       | BitNet rules + ACF parsing   | flight_prompt + regions  |
| User override        | Pin / manual score         | Manual tags                  | Prefer origin/dest       |
| Persistence          | `heuristics.json` overrides| `heuristics.json` aircraft_overrides | `heuristics.json` flight_* |
| “Learning”           | User pins → next sort uses | User tags → next tag uses    | Prefs + last success      |

*Prior to 3.1*, flight gen used only static data (prompt regex, `regions.json`, ICAO/seeds) and did not read or write user preferences. So:

- “Use this airport for Kenya” cannot be stored or reused.
- A successful “South Africa → Kenya” plan is not remembered to bias future similar requests.
- There is no place to persist “prefer HKJK for Kenya” or “last good origin for ZA”.

That is the **concrete** opportunity: reuse the same **override/preference + persistence** idea that BitNet already uses elsewhere.

---

## 3. Opportunity Assessment

### 3.1 Lightweight “learning” (no LLM)

**Idea**: Persist **flight-related preferences** and feed them into `generate_flight` so the system “learns” from user behavior or explicit choices.

Possible mechanisms:

1. **Region → preferred airport(s)**  
   - Store e.g. `flight_origin_prefs: BTreeMap<region_id, Vec<ICAO>>` and/or `flight_dest_prefs`.  
   - When building candidate_origins/candidate_dests for that region, **boost or pin** those ICAOs (e.g. prefer HKJK for KE).  
   - Source: explicit “Use this airport for Kenya” in the UI, or “Remember this” after a successful plan.

2. **Prompt sketch → last success**  
   - Store e.g. `last_success: Option<(prompt_normalized, origin_icao, dest_icao)>` or a small history.  
   - For similar prompts (e.g. same origin/dest region), **prefer** the last successful pair when they’re still in the candidate set.  
   - Source: record on successful plan generation.

3. **Failure avoidance**  
   - Optionally record “user rejected” or “no suitable” for a given prompt/region and avoid repeating the same bad suggestion (e.g. avoid suggesting an airport that was rejected).  
   - More UI and UX design; can be a later step.

**Where to store**

- **Option A – Extend `HeuristicsConfig`** in the same `heuristics.json`: add e.g. `flight_origin_prefs`, `flight_dest_prefs`, `flight_last_success` (and bump schema version).  
  - **Pros**: Single config file; BitNet model already loaded in GUI; same migration/backup story.  
  - **Cons**: HeuristicsConfig becomes shared between scenery, aircraft, and flight; need to keep schema and defaults clear.

- **Option B – Separate file** (e.g. `flight_preferences.json` in the same config dir).  
  - **Pros**: Flight gen stays independent of BitNet’s schema.  
  - **Cons**: Extra file and load/save from GUI; two “preference” systems.

**Recommendation**: Option A (extend HeuristicsConfig) is consistent with “BitNet as the place for user-driven overrides and preferences.” Flight gen would then take an optional `&HeuristicsConfig` (or a small `FlightPreferences` view) and use it when resolving origin/destination.

**Effort (rough)**

- Config extension + migration: small.  
- Passing prefs into `generate_flight` and applying “prefer this ICAO for region”: small–medium.  
- UI: “Use this airport for Kenya” / “Remember this flight”: medium (new messages and wiring).  
- No LLM, no new dependencies; fully offline and deterministic.

### 3.2 “AI-driven” in a stronger sense (optional LLM)

If we want the app to feel more “AI” (e.g. natural language, suggestions, explanations):

- **Option 1 – Local small LLM (e.g. BitNet-style or other)**  
  - Integrate an offline, small language model for understanding or rewriting prompts and maybe generating short descriptions.  
  - **Cost**: Model selection, inference (e.g. llama.cpp, burn, or similar), safety and latency; still no “learning” unless we add fine-tuning or RAG.

- **Option 2 – External API**  
  - Call an LLM API for parsing or suggestions.  
  - **Cost**: Network, keys, privacy; conflicts with current “offline-first” story.

- **Option 3 – Hybrid**  
  - Keep prompt parsing and airport selection rule-based (current + preferences).  
  - Add a small LLM or API only for **optional** “explain this flight” or “suggest a similar flight” in the UI.  
  - Learning would still be mainly via persisted preferences (Option A above), not model weights.

So: **we are not missing a necessary step by not having an LLM.** We are missing a **low-hanging** step: reusing BitNet’s override/preference pattern for flight gen so the system “learns” in the same way it already does for scenery and aircraft.

---

## 4. Summary and Recommendation

| Question | Answer |
|----------|--------|
| Can the system learn today? | **Scenery/aircraft**: yes (overrides in heuristics.json). **Flight gen**: no. |
| Do we have a structured world view? | Yes: regions, ICAO prefixes, pool (packs + base + seeds). Static. |
| Are we missing an opportunity with BitNet? | **Yes**: we have a persistence/override pattern in BitNet but do not use it for flight gen. |
| Is the opportunity “add a BitNet LLM”? | Optional and large. The **immediate** opportunity is **flight preferences in the existing BitNet config**, so flight gen “learns” like scenery and aircraft do. |

**Concrete next steps (if you want learning for flight gen)**

1. **Extend `HeuristicsConfig`** (or a dedicated sub-struct) with e.g. `flight_origin_prefs`, `flight_dest_prefs` (region → preferred ICAOs), and optionally `flight_last_success` (or a small history).
2. **Use these in `generate_flight`**: when resolving origin/destination by region, prefer (or pin) airports that appear in these maps when they’re in the candidate set.
3. **Persist**: save back to `heuristics.json` (same BitNet save path) when the user says “Use this airport for Kenya” or “Remember this flight” (and optionally on every successful generation if you want “last success” behavior).
4. **UI**: add actions in the Flight Gen tab to “Prefer this airport for [region]” and/or “Remember this flight” that update the config and call save.

That gives you **deterministic, offline, explainable “learning”** aligned with how BitNet already works—no LLM required. An LLM can be a later layer on top for language and suggestions if you decide you want it.

---

## Roadmap

- **3.1 (implemented):** Flight preferences in BitNet config (origin/dest prefs, last success; schema v10), wired into `generate_flight` and GUI: “Remember this flight”, “Prefer this origin”, “Prefer this destination”, and **Regenerate** (re-run same prompt for a new random outcome). The system now learns in the same way scenery pins and aircraft overrides do.
- **3.2 (planned):** Stronger “AI-driven” experience—e.g. optional local small LLM or API for natural language, suggestions, or explanations. To be pursued **after** 3.1 is in place and stable.

---

## Considering 3.2

With 3.1 in place, 3.2 is about **optional** “AI” layer on top of the existing rule-based + preference pipeline—not replacing it.

**What 3.2 could add**

- **Natural language**: Broader or fuzzy parsing (e.g. “somewhere sunny in the Med”, “weekend hop from the UK”) that today’s regex/region logic doesn’t cover.
- **Suggestions**: “Similar flight”, “longer/shorter version”, or “other airports in that region” driven by an LLM or a structured suggestion engine.
- **Explanations**: Short, natural-language summary of why a route or airport was chosen (e.g. for accessibility or debuggability).

**Options (from §3.2)**

| Option | Pros | Cons |
|--------|------|------|
| **Local small LLM** (e.g. llama.cpp, burn, small BitNet-style) | Offline, no keys, privacy-preserving; fits “no network” story. | Model choice, size, inference latency, packaging; no learning unless we add RAG/fine-tuning. |
| **External API** (e.g. OpenAI, Anthropic, local Ollama HTTP) | Strong NLU/suggestions with no model shipping. | Network, keys, privacy; conflicts with offline-first unless clearly optional. |
| **Hybrid** | Keep core (prompt → regions/ICAO, prefs, seeds) as today; add LLM/API only for **optional** “explain” / “suggest” / richer parsing. | Two code paths; need clear UX for “optional” (e.g. Settings toggle, or only when API/local model available). |

**Recommendation when pursuing 3.2**

- Treat the **current pipeline as canonical**: rule-based parsing + BitNet prefs + seeds/base. Any LLM/API should **augment** (e.g. rewrite or extend the user prompt, or add explanation/suggestions), not replace core generation.
- **Default off**: 3.2 features (local LLM or API) should be opt-in (e.g. Settings: “Use local model for suggestions”) so offline-only and no-key users are unchanged.
- **Scope incrementally**: e.g. first “Explain this flight” (one-shot summary from plan + regions), then “Suggest similar” or richer NL parsing, so each step is shippable and testable.

**History and local flavor (3.2)**

A nice addition within 3.2: **brief history** for the departure and destination airports, plus **pertinent points of history within ~10 miles** of each, to add context and flavor without changing the core flight plan.

- **Airport history**: Short, factual snippets for the origin and destination (founding, notable operations, wartime/civil use, etc.).
- **~10 mile radius**: A few “points of interest” or events (e.g. nearby battle, landmark, industry) so the flight feels grounded in place.

**Why it fits**

- Purely additive: no change to route or airport selection; it’s a presentation/context layer.
- Optional and cacheable: can be fetched once per airport (or per area) and stored; can be behind “Show history” or folded into “Explain this flight.”
- Good match for LLM or structured data: either an API/LLM summarizes from Wikipedia-style sources (with attribution), or we pull from a curated dataset (e.g. airport DB + POI DB) for offline use.

**Data / implementation options**

| Approach | Pros | Cons |
|----------|------|------|
| **Wikipedia / DBpedia (or similar API)** | Rich, up-to-date, many airports and places. | Network, rate limits, need summaries; licensing/attribution. |
| **Curated static bundle** | Offline, no keys, consistent tone. | Maintenance; only as good as the dataset (e.g. major airports + notable POIs). |
| **LLM summarization** (with cached results) | Flexible, one prompt per airport/area; can be local or API. | Same tradeoffs as general 3.2 LLM (offline vs API); need cache keyed by ICAO + radius so we don’t refetch every time. |

Recommendation: design the **UI and data shape first** (e.g. “History & context” block: origin snippet, origin ~10 mi points, destination snippet, destination ~10 mi points). Then choose data source based on whether 3.2 is local-first (curated or local LLM + cache) or API-allowed (Wikipedia/LLM API + cache). Keeping the 10-mile radius and “pertinent points” scope small keeps the feature manageable and avoids clutter.

---

## Implementation plan (3.2)

Phased plan so each step is shippable and testable. History/flavor is the first 3.2 feature; “Explain this flight” (LLM/API) can follow the same pipeline once the UI and data shape exist.

### Phase 1: Data shape and UI shell (no live data)

**Goal**: Types and a place in the GUI for “History & context” so we can add real data in Phase 2 without changing layout.

1. **Types (x-adox-core or x-adox-gui)**  
   - Introduce a small **flight context** type used only for display (core can stay agnostic, or core holds it if we want it in `FlightPlan`):
     - `AirportContext { icao, snippet: String, points_nearby: Vec<PointOfInterest> }`
     - `PointOfInterest { name, kind, snippet, distance_nm }` (or lat/lon if we need it for future map pins).
   - Add **optional** `context: Option<FlightContext>` to the data the GUI shows for a plan, where `FlightContext { origin: AirportContext, destination: AirportContext }`.  
   - Decision: either extend `FlightPlan` in `flight_gen.rs` with `pub context: Option<FlightContext>`, or keep `FlightPlan` as-is and have the GUI hold `Option<FlightContext>` alongside `current_plan`, filled later by a separate “load context” step. Latter keeps core free of 3.2-specific types; former keeps one struct for “everything about this plan.”

2. **GUI (flight_gen_gui.rs)**  
   - When `current_plan` is `Some`, show a **“History & context”** block (collapsible or always visible):
     - **Origin**: airport name + ICAO; below it, optional `origin_context.snippet` and then a short list of `points_nearby` (e.g. “• Point name — snippet”).
     - **Destination**: same structure.
   - If `context` is `None`, show a placeholder line: “No history loaded” or “Load history” (button in Phase 2). No network or data source yet.

3. **Deliverable**: Build and run; generate a flight; see the new block with placeholder/empty state. No new dependencies.

### Phase 2: Data source and loading

**Goal**: Fill `FlightContext` from a real source so “History & context” shows content.

**Option A – Curated static (offline-first)** *(Phase 2a implemented)*  
- **JSON file**: `flight_context.json` in the app config dir (same as `heuristics.json`). Keys are ICAO codes; each value has `snippet` and optional `points_nearby: [{ name, kind, snippet, lat, lon }]`. POIs are filtered to within 10 nm of the airport in code.  
- **Example**: `resources/flight_context.example.json` in the repo (copy to config dir as `flight_context.json` to enable).  
- Load in core (`load_flight_context(path, origin, destination)`); GUI calls it after each successful `generate_flight` and sets `plan.context`.  
- No network; no API keys.

**Option B – API (e.g. Wikipedia) or LLM** *(Phase 2b implemented)*  
- **“Fetch context”** button in Flight Gen: when enabled in Settings, fetches from a configurable **Context API URL** (GET `{url}/context/{icao}`), caches result as `flight_context_cache/{icao}.json`, then builds `FlightContext` (cache + curated). Next time the same airport is used, cache is read first.  
- **Settings**: “Fetch flight context from network” (default off) and “Context API URL” (e.g. `http://localhost:8080`). Stored in `app_config.json`.  
- API contract: response JSON is one airport’s `{ "snippet": "...", "points_nearby": [ { "name", "kind", "snippet", "lat", "lon" } ] }` (same as curated format).  
- Fetch runs in a background task (no UI freeze).

**Implementation steps**  
- **2a.** Implement **Option A** (curated JSON + 10 nm filter) so the feature works fully offline.  
- **2b.** (Optional) Add **Option B** behind a setting: “Fetch history from network” or “Use local model for context”; wire “Load history” to fetch/cache and then set `context` on the current plan.

**Deliverable**: With a small curated file containing at least one airport (e.g. EGLL), generate a flight from/to that airport and see real snippet + POIs in the block.

### Phase 3: Polish and “Explain this flight”

- **Cache invalidation**: If using API/LLM cache, consider TTL or “Refresh context” so users can force refetch.  
- **Attribution**: If data comes from Wikipedia or similar, show a short “Source: Wikipedia” (or similar) in the UI.  
- **Explain this flight**: Reuse the same “History & context” block (or a sibling section) for a one-shot **explanation** of why this route/aircraft was chosen. That can be a separate message/action that calls an LLM/API with plan summary and appends the result; same “optional and default off” as in Considering 3.2.

### File and crate touchpoints

| Area | Location | Change |
|------|----------|--------|
| Data shape | `crates/x-adox-core/src/flight_gen.rs` or new `crates/x-adox-core/src/flight_context.rs` | `FlightContext`, `AirportContext`, `PointOfInterest`; optional on `FlightPlan` or loaded separately. |
| Curated data | Config dir or `resources/` next to binary | `flight_context.json` (or per-ICAO files); 10 nm filter in code. |
| Cache (API/LLM) | Config dir, e.g. `flight_context_cache/` | Store by ICAO; read before fetch. |
| GUI state | `crates/x-adox-gui/src/flight_gen_gui.rs` | `Option<FlightContext>` next to or inside plan display; “History & context” block; “Load history” / “Fetch context” when Option B. |
| Main message routing | `crates/x-adox-gui/src/main.rs` | New message for “Load history” / “Fetch context” if async or delegated to a service. |

### Order of work (summary)

1. **Phase 1**: Types + “History & context” UI block with placeholder → merge, test in dev.  
2. **Phase 2a**: Curated JSON + load + 10 nm filter → feature complete for offline.  
3. **Phase 2b** (optional): API/LLM + cache + Settings toggle.  
4. **Phase 3**: Attribution, cache refresh, “Explain this flight” when you add LLM/API.

**No code change in this repo for 3.2 yet**—this section is the implementation plan when you start; Phase 1 can begin as soon as you’re ready.

---

## Re-assessment: History & context must be zero effort (user-friendly)

**What went wrong**

The current implementation (Phase 2a + 2b) assumes users will either:
- Manually copy/rename a JSON file into a “config folder”, or  
- Know what a “context server” is and paste a URL.

That is **not** zero effort. Typical users will not edit JSON or run servers. Exposing “context server”, “Context API URL”, or “put flight_context.json in config” in the UI was the wrong bar.

**What “zero effort” should mean**

- **Ideal**: User generates a flight; history & context appear (or one obvious action, e.g. “Show history”, with no setup).
- **Acceptable**: At most one simple choice, e.g. “Get airport history from the internet” (on/off), with a **fixed** service—no URLs, no JSON, no “server” wording.
- **Unacceptable**: Asking users to create/edit JSON, find a “config folder”, or supply any URL or server.

**Re-evaluated options**

| Approach | User effort | Realistic? |
|----------|-------------|------------|
| **Bundled data only** | Zero: ship a small default `flight_context` (e.g. top 50–200 airports) with the app. History “just works” for common airports. No Settings, no JSON, no network. | Yes. We maintain one bundle; app size grows slightly. |
| **Bundled + single “Fetch from internet”** | One toggle: “Get airport history from internet” (default off). When on, app calls **one fixed, well-known endpoint** (e.g. a service we host or a single public API). No URL box, no “context server”. | Yes if we have or create that endpoint; otherwise “bundled only” first. |
| **User-supplied JSON file** | High: user must find example, copy to config dir, rename. | **No** for general users. Keep only as an undocumented/advanced escape hatch if at all. |
| **User-supplied “context server” URL** | High: user must know what a server is and have a URL. | **No** for general users. Remove from primary UI; at most hide in “Advanced” and avoid the term “context server”. |

**Recommendation**

1. **Default to bundled data**: Ship a modest default flight-context dataset with the app (e.g. embedded or in `resources/` and loaded like the example). No config, no JSON steps—history appears for many airports out of the box.
2. **Simplify or remove Settings**: Remove “Context API URL” and “context server” from the main Settings UI. If we later add “Fetch from internet”, it should be a single checkbox that uses a **fixed** endpoint (no URL field).
3. **Stop instructing users to touch JSON**: Remove all copy about “add flight_context.json to config folder” and “example file in resources” from the UI. Power users can still add a file if the code supports it, but the UI should not require or suggest it.
4. **Before changing code**: Confirm with the product owner (you) which of these to implement first: (A) bundled default only, (B) bundled + one “Fetch from internet” toggle with a fixed endpoint, or (C) strip the current Settings to the minimum and add bundled data, then iterate.

**Next step**

Decide direction (A/B/C above or a variant). Then implement the chosen path and remove or hide the overengineered parts (URL box, JSON instructions, “context server” language) so history is zero effort for normal users.

---

## Option B: Research, assessment, and implementation plan

**Direction chosen**: Option B — bundled data + single “Fetch enhanced history from internet” toggle, with a **default known-good URL** (tested), **fallback** if primary is unavailable, and **documentation**. Users are OK with a URL only if we default to one that works; custom entry or backup URL is for when the primary source becomes unobtainable.

### 1. Research findings

**Wikipedia as source**

- **REST summary API** (no auth): `https://en.wikipedia.org/api/rest_v1/page/summary/{page_title}`  
  - Returns JSON: `extract` (plain intro, 1–3 sentences), `extract_html`, `title`, `description`. Ideal for airport “snippet”; no structured POIs (we can skip POIs for dynamic fetch or use extract only).
- **CORS**: CORS is a **browser** restriction. Our addon is a **native** HTTP client (ureq). We can call Wikipedia **directly** from the app; no CORS proxy required. This avoids dependency on third-party proxies (allorigins, corsproxy.io) for normal use.
- **Rate limits**: Wikipedia allows reasonable anonymous use; we cache per ICAO so repeat flights don’t refetch. If we ever hit limits, we can add a single fallback proxy URL in code (see Fallback below).

**ICAO → Wikipedia page title**

- **OurAirports `airports.csv`** (https://davidmegginson.github.io/ourairports-data/airports.csv) includes column **`wikipedia_link`** (e.g. `https://en.wikipedia.org/wiki/Heathrow_Airport`).  
- **Mapping**: `ident` (ICAO) → `wikipedia_link` → extract title from URL path (e.g. `Heathrow_Airport`). Many but not all airports have a link; rest get “No detailed history available” or bundled-only.

**Bundled data**

- Ship a modest **default** `flight_context` (e.g. 50–200 popular airports) so history works **out of the box** with zero setup. Format: existing `flight_context.json` shape (ICAO → snippet + optional points_nearby). Can be embedded (e.g. `include_str!`) or loaded from `resources/` at runtime.

**Fallback when primary is unobtainable**

- **Primary**: Direct Wikipedia REST API (no proxy).  
- **Fallback**: If we ever need it (e.g. Wikipedia blocks our client, or we add a proxy for rate limits), we can:  
  - (a) **Code-level fallback**: a second hardcoded URL (e.g. a known CORS proxy + Wikipedia) tried on primary failure, or  
  - (b) **Advanced setting**: single optional “Backup context URL” (hidden by default / in Advanced) so users or orgs can point to their own proxy if the primary is down.  
- **Documentation**: Document the feature (what it does, that “enhanced” uses Wikipedia, that we cache and respect “Fetch from internet” toggle). Document fallback only if we expose (b).

**Proxies (only if needed)**

- allorigins.win: ~20 req/min, often “dev only” in recommendations; returns 200 even on upstream errors.  
- corsproxy.io: prefix `https://corsproxy.io/?url=`.  
- cors.lol: `https://api.cors.lol/?url=`.  
- Prefer **direct Wikipedia** first; add proxy only as a configurable or hardcoded fallback if direct access fails in practice.

### 2. Assessment and feedback on the proposed design

**What works well**

- **Bundled first** → zero effort for common airports; no Settings needed for basic use.  
- **One checkbox** (“Enable enhanced history from Wikipedia”) with a **fixed** endpoint → no URL in main UI; we default to a known-good URL and test it.  
- **Cache per ICAO** → fast repeat use and offline after first fetch.  
- **Graceful fallback** copy: “No detailed history available — enjoy the flight!” when bundled and fetch both miss.  
- **Wording**: “Airport History & Trivia”, “works automatically for many airports”, “Uses a built-in service … Caches results” — all user-friendly and accurate.

**Refinements**

- **Default URL**: Use **direct Wikipedia REST API** as the default (no proxy). Test it from the addon (ureq) in CI or a manual test. If we ever need a proxy (e.g. rate limits), add a single **hardcoded** fallback URL in code and document “if primary is down we try backup”; still no user-facing URL.  
- **Custom / backup URL**: You said “if they choose a custom entry, or if the primary becomes unobtainable we’d have another way to get the data.” That can be:  
  - **Option 1**: Advanced/hidden “Backup context URL” (optional). When primary fails, app tries this URL (same contract: e.g. GET `{url}/context/{icao}` or we define a small contract). Power users can set a proxy or their own service.  
  - **Option 2**: No user-facing URL; we only ever use 1–2 hardcoded endpoints (primary + backup) and update them in a release if Wikipedia or proxy changes.  
  Recommend **Option 1** only if we want user/org-configurable fallback; otherwise **Option 2** keeps the UI minimal.  
- **OurAirports CSV**: Use it for ICAO → Wikipedia title. We can bundle a **processed** subset (ICAO → wikipedia_title or URL) in the app to avoid runtime CSV download and parsing; update the bundle periodically (e.g. at release time).  
- **POIs for dynamic fetch**: Wikipedia summary has no “points nearby.” For enhanced fetch we can: (a) show only snippet (no POIs), or (b) try to parse a “History” section via a second API call. (a) is simpler and consistent with “short history”; recommend (a) for v1.

### 3. Implementation plan

**Phase 1: Bundled default (zero effort)**

1. **Bundle a default `flight_context`**  
   - Build or curate a single dataset (e.g. 50–200 airports) in existing `flight_context.json` format (snippet + optional points_nearby).  
   - Ship it with the app (e.g. `resources/flight_context.json` or embedded).  
   - **Load order**: When building `FlightContext` for a plan, always try **bundled** first; if both origin and destination have data, use it. No Settings required.

2. **Remove/hide overengineered UI**  
   - Remove “Context API URL” and “context server” from the main Settings screen.  
   - Remove copy that tells users to “add flight_context.json to config folder” or “example file in resources.”  
   - Keep “Fetch context” (or “Load history”) in Flight Gen only if we need it for enhanced fetch; otherwise history can appear automatically from bundle.

3. **Optional: config file override**  
   - If we keep support for a user `flight_context.json` in config dir, do not mention it in the UI; treat as undocumented override. Load order: bundled → config file (if present) → per-airport override from cache/enhanced fetch.

**Phase 2: Enhanced fetch (one toggle, default URL, tested)**

4. **OurAirports → Wikipedia title mapping**  
   - Either: (a) bundle a preprocessed map (ICAO → Wikipedia page title) derived from OurAirports `airports.csv` (wikipedia_link column), or (b) bundle the CSV and parse at startup for the subset we need. (a) is smaller and faster; recommend (a).  
   - When we need a snippet for an ICAO not in bundled data: look up title; if missing, show “No detailed history available.”

5. **Wikipedia summary fetch**  
   - **Default primary URL**: `https://en.wikipedia.org/api/rest_v1/page/summary/{title}` (title from step 4, URL-encoded). Call directly from the app (no proxy).  
   - Parse JSON; take `extract` (and optionally `description`) as the snippet; no POIs for dynamic fetch.  
   - Cache result per ICAO in existing cache dir (e.g. `flight_context_cache/{icao}.json`) in the same format we use for curated/bundled so one code path displays it.

6. **Settings: single checkbox**  
   - One option: **“Enable enhanced history from Wikipedia (for more airports)”** — default off (or on after testing; your choice).  
   - Helper text: “Uses a built-in service to fetch summaries. Caches results for speed and offline use. Covers gaps in built-in data.”  
   - No URL field in main UI.

7. **Flow**  
   - On flight generation (or on “Fetch context” if we keep that button):  
     - Try bundled for origin and destination.  
     - For any airport missing from bundle: if “enhanced” is on, try cache; on cache miss, fetch from primary URL (Wikipedia); on success, write cache and show snippet.  
     - If fetch fails (network, 404, rate limit): optionally try **fallback** URL if configured (see step 8); else show “No detailed history available — enjoy the flight!”

8. **Fallback and optional custom URL**  
   - **Primary**: Direct Wikipedia (step 5).  
   - **Fallback**: Either (A) one hardcoded backup (e.g. proxy + Wikipedia) in code, tried on primary failure, or (B) advanced/hidden “Backup context URL” stored in config; when primary fails, GET `{backup_url}/context/{icao}` (or agreed contract). Document only if (B) is exposed.  
   - If “they choose a custom entry” means “user can type a backup URL”: implement (B) in Advanced and document. Otherwise (A) is enough.

**Phase 3: Test and document**

9. **Test**  
   - **Bundled**: Generate flight between two airports in the bundle; confirm “History & context” shows without network.  
   - **Enhanced**: Turn on “Enable enhanced history from Wikipedia”; generate flight for an airport not in bundle; confirm fetch, cache, and display.  
   - **Offline**: Disable network; confirm bundled + cached still work and message for uncached is friendly.  
   - **Primary failure**: Simulate primary unavailable (e.g. wrong URL or mock); confirm fallback (if implemented) or graceful message.

10. **Documentation**  
    - User-facing: Short note (e.g. in Settings or in-app help): “Airport History & Trivia adds short historical notes and nearby facts to generated flights when available. Works automatically for many airports. Enable ‘Enhanced history from Wikipedia’ for more airports; results are cached.”  
    - If we add advanced “Backup context URL”: one sentence and the expected URL contract (e.g. GET returns JSON with snippet + optional points_nearby).  
    - Dev/maintainer: Where the default (and fallback) URLs are defined, how to update the bundled dataset, and how to add a new fallback in a future release if the primary becomes unobtainable.

### 4. Summary

| Item | Decision |
|------|----------|
| **Default data** | Bundled `flight_context` (50–200 airports); no user setup. |
| **Enhanced fetch** | One checkbox; fixed primary URL = direct Wikipedia REST API (tested). |
| **CORS** | Not needed for native app; call Wikipedia directly. |
| **ICAO → Wikipedia** | OurAirports `wikipedia_link`; bundle processed map (ICAO → title). |
| **POIs for dynamic** | Omit for v1 (snippet only from summary API). |
| **Fallback** | Primary failure → hardcoded backup URL and/or optional “Backup context URL” in Advanced; document if user-configurable. |
| **Settings** | Remove URL and “context server”; one checkbox + short copy. |
| **Test** | Bundled, enhanced, offline, primary-failure paths. |
| **Docs** | User: what the feature does and that enhanced uses Wikipedia + cache; Dev: where URLs live and how to update/fallback. |

This plan implements Option B with a known-good default URL, tested behavior, fallback for unobtainable primary, and documentation—without asking normal users to edit JSON or supply a URL unless we expose an optional backup in Advanced.

### 5. Implementation status (Option B)

- **Phase 1**
  - **Bundle (50–200 airports)**: `crates/x-adox-core/data/flight_context_bundle.json` — 63 airports, embedded via `include_str!`. Load order: cache → bundled → config. Script: `scripts/generate_flight_context_bundle.py` reads `icao_to_wikipedia.json`, fetches Wikipedia summary for each ICAO, writes the bundle (run from repo root; requires network).
  - Settings: “Airport History & Trivia” section with one checkbox “Enable enhanced history from Wikipedia (for more airports)” and helper text; no URL field or “context server” copy.
  - Generate/Regenerate and “Fetch context” load from bundled + config + cache; when “enhanced” is on, auto-fetch after generate and on “Fetch context” for airports not in bundle/cache. Empty context shows “No detailed history available — enjoy the flight!”
- **Phase 2**
  - ICAO → Wikipedia: **runtime** from embedded `icao_to_wikipedia.csv` (ident,title). Generated from OurAirports `airports.csv` via `scripts/generate_icao_to_wikipedia_from_ourairports.py` (~16k entries); script writes both `.json` and `.csv`; app parses CSV at first use (`get_icao_to_wikipedia()`).
  - Wikipedia fetch: primary `https://en.wikipedia.org/api/rest_v1/page/summary/{title}`; parse `extract`, cache as `AirportContextFile` (snippet only, no POIs). User-Agent set. GUI: `fetch_airport_context_from_wikipedia`, `load_or_fetch_flight_context_blocking`.
  - **Fallback**: On primary request failure, try hardcoded proxy `https://api.allorigins.win/raw?url=` + encoded primary URL (same Wikipedia API). See §6 for where to change it.
- **Phase 3**
  - **Tests**: `test_load_flight_context_from_json` (in `flight_gen.rs`) verifies: curated JSON load, bundled has ≥50 airports and EGLL/LIRF, `load_flight_context_with_bundled`, `get_icao_to_wikipedia`. Manual: generate flight EGLL→LIRF (bundled, no network); enable enhanced, generate for airport not in bundle, confirm fetch/cache/display; offline use cached + bundle; primary failure falls back to proxy or shows friendly message.
  - **Docs**: In-app copy in Settings. Dev/maintainer details in §6.

### 6. Dev/maintainer: URLs, bundle, fallback

- **Primary URL**: `WIKIPEDIA_SUMMARY_URL` in `crates/x-adox-gui/src/flight_gen_gui.rs` (`https://en.wikipedia.org/api/rest_v1/page/summary`). Used as `{base}/{urlencoding::encode(title)}`.
- **Fallback**: `WIKIPEDIA_FALLBACK_PROXY_BASE` in the same file (`https://api.allorigins.win/raw?url=`). When the primary GET fails (network, rate limit, etc.), the app tries `fallback_base + urlencode(primary_url)` and parses the same Wikipedia summary JSON. To use a different proxy or remove fallback, change or remove that constant and the `Some(WIKIPEDIA_FALLBACK_PROXY_BASE)` passed into `fetch_airport_context_from_wikipedia`.
- **Bundled dataset**: `crates/x-adox-core/data/flight_context_bundle.json`. To update or expand: (1) Regenerate ICAO map: `python3 scripts/generate_icao_to_wikipedia_from_ourairports.py` (downloads OurAirports CSV, writes `icao_to_wikipedia.json` and `icao_to_wikipedia.csv`; app embeds and parses the CSV at runtime). (2) Optionally run `python3 scripts/generate_flight_context_bundle.py` to refresh the 63-airport bundle (requires network). (3) Commit updated data. The bundle fetches `extract` only; for **10 nm POI historical text** use the curated overlay: `crates/x-adox-core/data/flight_context_pois_overlay.json` — add `{ "ICAO": [ { "name", "kind", "snippet", "lat", "lon" }, ... ] }`; merged at load time and filtered by 10 nm.
- **Automated POIs (Wikipedia geosearch):** When enhanced history is on, the GUI calls Wikipedia’s `geosearch` API (radius 10 km, limit 20) for origin and destination lat/lon, passes results as dynamic POIs into `load_flight_context_with_bundled`. Core merges overlay + dynamic POIs and filters to 10 nm. Cache: `flight_context_cache/pois_near/{lat:.3}_{lon:.3}.json`, TTL 7 days. See `fetch_pois_near_from_wikipedia` in `crates/x-adox-gui/src/flight_gen_gui.rs`.
- **POI descriptions (travelogue-style):** The first 5 geosearch POIs per airport are enriched with the Wikipedia summary extract (cached per title under `flight_context_cache/poi_extract/`, TTL 7 days) so "Surrounding Landmarks" shows short descriptions, not just titles.
- **History when empty:** The "History" section is always shown; when no snippet is available it displays a short message suggesting the user enable enhanced history and click "Fetch context".
- **Optional “Backup context URL”**: Config still stores `flight_context_api_url` for round-trip. A future Advanced UI could expose it as a custom proxy base (same contract: append encoded primary URL and return Wikipedia summary JSON) or as a different context API.

### 7. Evaluation: "Scalability wall" and "missing lore"

This section evaluates the critique that the current implementation is an oversimplified bottleneck and that two failures need fixing: the "scalability wall" and the "missing lore" (10-mile POIs).

**1. "The Scalability Wall: curated, manual ICAO-to-Wikipedia map is a dead end."**

- **Accuracy**: Partially off. The ICAO → Wikipedia map is **not** manually curated: it is generated by `scripts/generate_icao_to_wikipedia_from_ourairports.py` from OurAirports `airports.csv` (`ident` + `wikipedia_link`). That yields ~16k entries; the script is the source of truth, not hand-edited JSON.
- **Valid ask**: The critique's real point is **runtime resolution**: "use wikipedia_link directly from airports.csv at runtime" so the app does the mapping itself and doesn't depend on a pre-baked JSON. Right now we do: run script (or CI) → embed `icao_to_wikipedia.json` → app uses embedded data. So the app never talks to OurAirports; it uses a snapshot.
- **To "fix it properly"**: Either (a) **ship `airports.csv`** (or a slim CSV with `ident,wikipedia_link`) in the app and parse it at startup to build the map in memory, or (b) **download** OurAirports CSV on first use / periodically and build the map. (a) = no network, but larger asset and still a snapshot; (b) = always up to date when online, but latency, failure handling, and possibly caching. Both are viable; the current "embed script output" is a third option that avoids shipping CSV and avoids runtime download.

**2. "The Missing Lore: 10-mile POIs were the utility; removing them turned it into a basic Wikipedia reader. Need a dedicated 'Surrounding Landmarks' section."**

- **Accuracy**: POIs were **restored** in code and data: 10 nm filtering in `build_airport_context`, overlay `flight_context_pois_overlay.json` merged at load time, and the UI shows "• Name — snippet (X.X nm)" under Origin/Destination. So they are not "removed." Two gaps remain:
  - **Scale**: The overlay is a small curated set (e.g. EGLL, LIRF). There is no automated source for "landmarks within 10 nm" for every airport; the bundle and Wikipedia fetch only provide the airport **snippet**, not POIs.
  - **UI**: Landmarks are mixed under "Origin" / "Destination" with no distinct heading. A **dedicated "Surrounding Landmarks (within 10 nm)"** subsection would make the 10-mile lore visible and intentional instead of "some bullets under the snippet."
- **To "get it back on track"**: (1) **UI**: Add an explicit "Surrounding Landmarks" block (or subheading) in the History & context panel that lists `points_nearby` for origin and destination, with the 10 nm boundary stated. (2) **Data**: Keep the overlay for now; document that adding entries there is how you get more landmarks. A future step could be a separate data source or API for "POIs near lat/lon" (e.g. Wikipedia geo, or another provider) if we want this to scale without hand-maintained overlay.

**Summary**

| Critique | Reality | Concrete next step if we adopt the ask |
|----------|--------|----------------------------------------|
| Scalability = dead end | Map is script-generated from OurAirports (~16k), not manual. | Move to **runtime** mapping: ship minimal CSV or download OurAirports CSV and build ICAO→title at startup/first use. |
| Lore removed / no utility | 10 nm POIs are in code + overlay; shown as bullets under Origin/Dest. | **UI**: Dedicated "Surrounding Landmarks (10 nm)" section. **Data**: Expand overlay and/or add automated POI source later. |

**Recommendation**

1. **Do first (low effort, high clarity):** Add a dedicated **"Surrounding Landmarks (within 10 nm)"** subsection in the History & context panel. Same data as today; just give it a visible heading and list origin/destination `points_nearby` there. That makes the 10-mile lore the intended feature instead of hidden bullets.
2. **Optional next (if you want the app to own scaling):** Move ICAO→Wikipedia to runtime—e.g. ship a slim `ident,wikipedia_link` CSV (or download OurAirports CSV on first launch), parse once, build the map in memory. Then the app doesn’t depend on a pre-baked JSON snapshot.
3. **Later:** Expand the POI overlay and/or add an automated POI source (e.g. Wikipedia geo/nearby) if you want landmarks to scale without hand-maintained JSON; that’s a larger change.

**Done (implementation):** Steps 1, 2, and 3 are implemented. **Automated POI source:** When “Enable enhanced history from Wikipedia” is on, the GUI fetches **Wikipedia geosearch** (API `list=geosearch`, 10 km radius, 20 results) for origin and destination coordinates, converts results to `PoiFile`, and passes them into `load_flight_context_with_bundled` as dynamic POIs. Core merges overlay + dynamic POIs and filters all to 10 nm. Geosearch results are cached under `flight_context_cache/pois_near/{lat:.3}_{lon:.3}.json` with a 7-day TTL. Overlay still provides hand-picked landmarks (e.g. Windsor Castle, Ostia Antica); geosearch adds scalable nearby Wikipedia pages for any airport with lat/lon. **Context** in the panel means both: (1) **History** of the airport itself (outline/snippet from Wikipedia or bundle), and (2) **Surrounding Landmarks (within 10 nm)** — POIs from the overlay. The UI now shows explicit "History" and "Surrounding Landmarks (within 10 nm)" subheadings per origin/destination. ICAO→Wikipedia is built at runtime from embedded `icao_to_wikipedia.csv` (OurAirports-derived, script outputs both JSON and CSV); no pre-baked JSON in the app.

**Solutions for wrong or missing landmarks (no blocklists):**

- **Wrong name or description:** Landmarks come from Wikipedia (geo-tagged articles). If an article is inaccurate (e.g. wrong historic name), the right fix is to **edit the article on Wikipedia** so the correction benefits everyone. The UI attribution says: "Wrong or missing? Edit the article on Wikipedia, or add landmarks to the overlay (see docs)." In effect, users of the app can give Wikipedia a **user audit**: e.g. articles sometimes use internal or official names that never caught on (e.g. Air Ministry paperwork names) instead of what people actually called the place (e.g. **RAF Rochford** for Southend in WW2); correcting that at the source improves the encyclopedia for everyone.
- **Missing famous places:** Many notable places lack coordinates or a geo-tagged Wikipedia article, so they never appear in geosearch. The **curated overlay** is a stopgap (add entries to `flight_context_pois_overlay.json`); it is not scalable and erodes trust when users notice important landmarks missing and obscure ones present. A scalable fix is to add **Wikidata** (and optionally semantic/importance-aware selection) — see below.
- **Same-location historic items:** POIs at 0.0 nm (e.g. an article about the airfield itself) are **shown**; there is no minimum-distance filter, so historic same-site entries remain visible.

### 8. Surrounding Landmarks: no semantic geosearch, no Wikidata (trust and scalability)

**What we do today**

- **Only** Wikipedia’s `list=geosearch`: one request per airport (origin/destination) with `gscoord`, `gsradius=10000` (10 km), `gslimit=20`, no ranking by importance or type. Results are whatever 20 geo-tagged articles the API returns (order not guaranteed by notability). We then filter to 10 nm in core and enrich the first 5 with Wikipedia summary extract.
- **No semantic geosearch:** We do not filter or rank by “landmark type” (stadium, pier, museum, tourist attraction). So we can get Catholic United F.C. but not Southend United; we can get Rochford Rural District but not Southend Pier — because the latter’s Wikipedia article may lack coordinates in the geo index or fall outside the arbitrary 20-result cap.
- **No Wikidata:** We do not query Wikidata. Wikidata has structured coordinates (P625), types (P31: stadium, pier, museum, etc.), and links to Wikipedia; the **Wikidata Query Service** (SPARQL) supports `wikibase:around` (radius in km) and filters by instance-of, so we could request “stadiums, piers, tourist attractions, landmarks within 10 nm” and merge with Wikipedia for labels/extracts. That would be scalable and semantic.

**Why this erodes trust**

- Relying on “users can add missing landmarks to the overlay” is not scalable and only fixes a few airports; it also signals that we don’t prioritise “what’s important” — so users reasonably ask what else is missing.
- Without semantic or importance-aware selection, the list feels arbitrary: minor clubs and administrative districts appear, while well-known landmarks (Southend Pier, Roots Hall/Southend United) do not, even when they are well within 10 nm. That undermines confidence in the feature.

**Scalable direction (for implementation)**

1. **Add Wikidata as a second source**
   - Query **Wikidata Query Service** (or Wikidata API) for entities with coordinates (P625) within ~10–20 km of the airport, optionally filter by P31 (instance of: stadium Q483110, pier Q337234, museum Q33506, tourist attraction Q570116, landmark Q2319498, etc.), and require/enrich with English Wikipedia sitelink so we can show the same extract we use today. Merge with Wikipedia geosearch results and deduplicate (e.g. by Wikipedia title or Q-id). Cache similarly (e.g. by lat/lon bucket, TTL 7 days).
2. **Optionally rank by importance**
   - Use sitelink count or Wikidata “importance” so more notable places sort higher; or simply merge Wikidata results (which are type-filtered) with Wikipedia results and dedupe, then sort by distance. That already improves relevance without a full “semantic” ranking model.
3. **Keep overlay as override**
   - The overlay remains for local corrections and one-offs; the primary, scalable story is “Wikipedia geosearch + Wikidata (semantic/type-aware), merged and filtered to 10 nm.”

**Where it plugs in**

- **GUI:** `fetch_pois_near_from_wikipedia` in `crates/x-adox-gui/src/flight_gen_gui.rs` ; `fetch_pois_near_from_wikidata` is also used. In `load_or_fetch_flight_context_blocking` we call both, merge and dedupe, then pass combined dynamic POIs into `load_flight_context_with_bundled`. Core already merges overlay + dynamic and filters to 10 nm; no core change required.
- **Attribution:** UI line:  “From Wikipedia and Wikidata (geo-tagged). Wrong or missing? Edit on Wikipedia or add to the overlay (see docs)."

**Done (implementation):** Wikidata is implemented. `fetch_pois_near_from_wikidata` queries the Wikidata Query Service (SPARQL) with `wikibase:around` (20 km), filters by P31/P279* to types: stadium (Q483110), pier (Q337234), museum (Q33506), tourist attraction (Q570116), landmark (Q2319498), requires English Wikipedia sitelink, returns up to 30 items. Cached under `flight_context_cache/pois_near_wikidata/{lat:.3}_{lon:.3}.json` (TTL 7 days). `merge_pois_dedupe_by_title` merges Wikipedia + Wikidata and dedupes by title (Wikipedia first). First 8 merged POIs enriched with Wikipedia extract.

**Unresolved: "Surrounding Landmarks" empty for some users.** Despite multiple attempts (coordinate fallbacks, no empty cache write, empty-cache treated as miss, 30s fetch timeout, build_airport_context fallback coords, status messaging, log::warn on empty fetch), the bug where "Surrounding Landmarks (within 10 nm)" remains empty after "Fetch context" for e.g. EGMC (London Southend) was not fixed. The app writes a log file that should be used for debugging: **`x-adox.log`** in the config directory (e.g. Linux: `~/.config/x-adox/x-adox.log`; Windows: `%APPDATA%\x-adox\`; see config root in CLAUDE.md). Consult that log when investigating fetch failures, network errors, or empty POI results instead of relying on code inspection alone.
