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
