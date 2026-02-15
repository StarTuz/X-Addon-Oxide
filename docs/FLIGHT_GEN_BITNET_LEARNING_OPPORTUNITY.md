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

**Option A – Curated static (offline-first)**  
- Add a **JSON bundle** (e.g. `flight_context.json` or a small `flight_context/` directory with one file per ICAO or per region) in the app’s data/config or shipped next to the binary.  
- Schema: key by ICAO; each value has `snippet` and optional `points_nearby: [{ name, kind, snippet, lat, lon }]` (filter by 10 nm in code when we have airport lat/lon).  
- Load in GUI or in a small helper in core: given `FlightPlan` (origin/dest ICAO + lat/lon), look up context for both airports, filter POIs by distance, build `FlightContext`.  
- No network; no API keys. Good first step.

**Option B – API (e.g. Wikipedia) or LLM**  
- Add an optional “Load history” (or “Fetch context”) action that calls an API or local LLM, then caches result by ICAO (and maybe lat/lon bucket for POIs) under config dir, e.g. `flight_context_cache/{icao}.json`.  
- Same `FlightContext` shape; populate from API/LLM response and write cache. Next time, read from cache if present.  
- Requires Settings toggle and possibly API key config; keep default off.

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
