# NLP Dictionary Reference

The NLP dictionary (`nlp_rules.json`) lets you teach the flight generator custom vocabulary without touching code. Access it via **Edit Dictionary** in the Flight Gen tab. Click **▶ Valid Values Reference** inside the editor for an inline cheatsheet.

## Schema (v2)

```json
{
  "aircraft_rules":    [ ... ],
  "time_rules":        [ ... ],
  "weather_rules":     [ ... ],
  "surface_rules":     [ ... ],
  "flight_type_rules": [ ... ],
  "duration_rules":    [ ... ],
  "schema_version": 2
}
```

All categories are optional — omit any that you don't need.

---

## Common Fields (all rule categories)

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | yes | Human-readable label (display only) |
| `keywords` | string[] | yes | Phrases that trigger this rule. Word-boundary matched, case-insensitive. |
| `mapped_value` | string | yes | The engine value this rule maps to. See each category for valid options. |
| `priority` | number | no | Higher = checked first within this category. Default `0`. Use to ensure specific rules beat broader ones. |

---

## `aircraft_rules`

Maps custom phrases to aircraft category tags used by the aircraft picker.

**Extra fields (aircraft_rules only):**

| Field | Type | Description |
|---|---|---|
| `min_distance_nm` | number | Soft distance floor (nm). Overridden by duration keywords. |
| `max_distance_nm` | number | Soft distance cap (nm). Overridden by duration keywords. |
| `speed_kts` | number | Cruise speed override (knots) for distance↔duration math. Overrides the category heuristic. |

**`mapped_value`:** Free-form tag matched against your aircraft library (e.g. `"General Aviation"`, `"Jet"`, `"Turboprop"`). Any non-empty string is valid.

**Duration keyword priority:** Explicit phrasing like `"2 hour flight"` or `"short"` always overrides `min/max_distance_nm`. The aircraft rule constraints only apply when no duration keyword is present.

**Speed heuristic defaults** (when `speed_kts` is absent):

| Tag in aircraft library | Default speed |
|---|---|
| heavy, airliner | 450 kts |
| jet | 350 kts |
| turboprop | 250 kts |
| helicopter, helo, seaplane, float | 100 kts |
| anything else (GA) | 120 kts |

### Example

```json
{
  "name": "Puddle Jumper",
  "keywords": ["puddle jumper", "small plane", "tiny plane"],
  "mapped_value": "General Aviation",
  "max_distance_nm": 400,
  "speed_kts": 110
}
```

---

## `time_rules`

Maps custom phrases to a solar time-of-day filter. Only airports currently in that solar phase are considered.

**Valid `mapped_value` options:**

| Value(s) | Maps to |
|---|---|
| `dawn` · `sunrise` · `morning` · `golden hour` · `golden` | Dawn (05:00–07:00 local solar) |
| `day` · `daytime` · `daylight` · `afternoon` · `noon` | Day (08:00–17:00) |
| `dusk` · `sunset` · `evening` · `twilight` · `civil twilight` | Dusk (18:00–19:00) |
| `night` · `midnight` · `dark` · `night flight` · `moonlight` · `late night` | Night (20:00–04:00) |

**Built-in aliases** (hardcoded, no JSON entry needed): *dawn, sunrise, morning, golden hour, day, daytime, daylight, afternoon, noon, dusk, sunset, evening, twilight, night, midnight, dark*

Use `time_rules` to add your own aliases (e.g. `"blue hour"` → `"dusk"`).

---

## `weather_rules`

Maps custom phrases to a live-METAR weather filter. Requires network access; if METARs are unavailable the constraint is skipped and no weather label is shown in the flight summary.

**Valid `mapped_value` options:**

| Value(s) | Maps to |
|---|---|
| `clear` · `sunny` · `fair` · `vfr` · `cavok` · `blue sky` · `cavu` · `scenic` | Clear |
| `cloudy` · `overcast` · `clouds` · `mvfr` · `marginal` · `scattered` · `broken` | Cloudy |
| `storm` · `thunder` · `thunderstorm` · `severe` · `lifr` · `low ifr` | Storm |
| `gusty` · `windy` · `breezy` · `turbulent` · `gusts` | Gusty |
| `calm` · `still` · `smooth` · `glassy` · `light winds` | Calm |
| `snow` · `blizzard` · `ice` · `wintry` · `winter` · `frozen` · `snowy` · `icy` | Snow |
| `rain` · `showers` · `wet` | Rain |
| `fog` · `mist` · `haze` · `ifr` · `instrument` · `smoky` | Fog |

---

## `surface_rules`

Maps custom phrases to runway surface type constraints.

**Valid `mapped_value` options:**

| Value(s) | Maps to | Effect |
|---|---|---|
| `soft` · `grass` · `dirt` · `gravel` · `strip` · `unpaved` | Soft | Prefers grass/dirt/gravel runways |
| `hard` · `paved` · `tarmac` · `concrete` · `asphalt` | Hard | Prefers paved runways |
| `water` · `seaplane` · `float` | Water | Seaplane bases only |

**Built-in keywords** (hardcoded, no JSON entry needed): *grass, dirt, gravel, strip, unpaved → Soft | paved, tarmac, concrete, asphalt → Hard | water, seaplane, floatplane, amphibian → Water*

---

## `flight_type_rules`

Maps custom phrases to flight-type constraints.

**Valid `mapped_value` options:**

| Value(s) | Maps to | Effect |
|---|---|---|
| `bush` · `backcountry` · `remote` · `stol` | Bush | Remote strips, also implies Soft surface if not set |
| `regional` · `commuter` | Regional | Standard airports |

**Built-in keywords** (hardcoded): *bush, backcountry → Bush | regional → Regional*

---

## `duration_rules`

Maps custom phrases to distance-range envelopes. JSON rules are checked before hardcoded keywords.

**Valid `mapped_value` options:**

| Value(s) | Maps to | Distance range |
|---|---|---|
| `short` · `hop` · `quick` · `sprint` | Short | 10–200 nm |
| `medium` · `mid` | Medium | 200–800 nm |
| `long` · `long range` | Long | 800–2500 nm |
| `haul` · `long haul` · `ultra long` · `intercontinental` · `transatlantic` · `transpacific` · `transcontinental` | Haul | 2500–12000 nm |

> **Tip:** Give multi-word keywords higher `priority` so they beat single-word ones. E.g. `"long haul"` at priority `1` beats `"long"` at priority `0`, preventing "long haul" from being caught as just "long".

---

## Priority System

All categories support `priority` for intra-category ordering. Higher number = checked first. Rules with the same priority are checked in JSON order (top to bottom).

```json
{ "name": "Long Haul", "keywords": ["long haul"], "mapped_value": "Haul", "priority": 1 },
{ "name": "Long",      "keywords": ["long"],       "mapped_value": "Long", "priority": 0 }
```

---

## Complete Example

```json
{
  "aircraft_rules": [
    {
      "name": "Puddle Jumper",
      "keywords": ["puddle jumper", "small plane"],
      "mapped_value": "General Aviation",
      "max_distance_nm": 400,
      "speed_kts": 110,
      "priority": 0
    },
    {
      "name": "Heavy Metal",
      "keywords": ["heavy metal", "jumbo"],
      "mapped_value": "Civilian Airliner",
      "min_distance_nm": 500,
      "speed_kts": 480,
      "priority": 0
    }
  ],
  "time_rules": [
    {
      "name": "Blue Hour",
      "keywords": ["blue hour", "magic hour"],
      "mapped_value": "dusk",
      "priority": 0
    }
  ],
  "weather_rules": [
    {
      "name": "The Soup",
      "keywords": ["soup", "low vis", "pea soup"],
      "mapped_value": "fog",
      "priority": 0
    }
  ],
  "surface_rules": [
    {
      "name": "Outback Strip",
      "keywords": ["outback", "red dirt", "station"],
      "mapped_value": "soft",
      "priority": 0
    }
  ],
  "flight_type_rules": [
    {
      "name": "Remote Bush",
      "keywords": ["remote", "outback", "wilderness"],
      "mapped_value": "bush",
      "priority": 0
    }
  ],
  "duration_rules": [
    {
      "name": "Sprint",
      "keywords": ["sprint", "quick hop"],
      "mapped_value": "short",
      "priority": 1
    }
  ],
  "schema_version": 2
}
```

---

## Validation

The editor validates `mapped_value` on save for all categories except `aircraft_rules` (where it's a free-form tag). If you use an unrecognized value, the save is blocked and an error message lists the valid options. JSON syntax errors are also reported with the line number from the parser.
