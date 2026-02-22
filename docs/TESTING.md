# Test Coverage Reference

This document maps every test file and test function to the feature it protects.
Update this file whenever tests are added, removed, or restructured.

---

## Workspace Summary

| Crate | Unit tests | Integration/regression tests | Total |
|---|---|---|---|
| `x-adox-bitnet` | 60 | 7 | 67 |
| `x-adox-core` | 26 | ~115 | ~141 |
| `x-adox-gui` | 0 | 0 | 0 (visual only) |
| `x-adox-cli` | 0 | 0 | 0 |
| **Total** | **86** | **~122** | **~208** |

---

## x-adox-bitnet

### `src/lib.rs` — 29 unit tests

**Feature: Aircraft classification (BitNet heuristics engine)**

| Test | Protects |
|---|---|
| `test_predict_panc` | SimHeaven packs not misclassified as aircraft |
| `test_predict_simheaven_consistency` | Scenery pack names not producing aircraft tags |
| `test_predict_tags_airliner` | Wide-body airliner tag assignment |
| `test_predict_tags_bizjet` | Business jet tag assignment |
| `test_predict_tags_ga_piston` | GA piston tag assignment |
| `test_predict_tags_ga_turboprop` | Turboprop GA tag assignment |
| `test_predict_tags_military_jet` | Military jet tag assignment |
| `test_predict_tags_concorde` | Historical supersonic classification |
| `test_predict_tags_trident` | Historical tri-jet classification |
| `test_predict_tags_bizjet_is_ga` | Small bizjet → GA fallback |
| `test_predict_tags_generic_cargo_jet` | Generic cargo → Jet tag |
| `test_predict_tags_airways_express` | Obscure airline name classification |
| `test_predict_tags_unknown_jet_safety_net` | Unknown jet → safety net tag |
| `test_predict_tags_unknown_prop` | Unknown prop → GA tag |
| `test_predict_tags_historical_prop_airliner` | DC-3/Ford Trimotor classification |
| `test_predict_tags_historical_bomber` | WWII bomber classification |
| `test_predict_tags_modern_bomber` | Modern military bomber classification |
| `test_predict_tags_regional_turboprop` | ATR/Dash-8 regional classification |
| `test_predict_tags_new_manufacturer_lockheed` | Lockheed-Martin classification |
| `test_predict_tags_helicopter_specific` | Helicopter type tag |
| `test_predict_tags_vulcan_bomber` | Vulcan → military bomber |
| `test_predict_tags_il76_cargo` | Ilyushin cargo classification |
| `test_predict_tags_pc12_ga` | PC-12 → GA turboprop |
| `test_predict_tags_with_acf_parsing` | `.acf` file parsing for classification |
| `test_predict_tags_boeing_707` | Boeing 707 classification |
| `test_predict_tags_standalone_707` | "707" alone → Boeing tag |
| `test_predict_tags_airbus_standalone_320` | "A320" alone → Airbus tag |
| `test_predict_tags_manual_override` | Manual category override persists |
| `test_predict_tags_cirrus_sf50_no_fokker` | SF50 not confused with Fokker |

### `src/flight_prompt.rs` — 28 unit tests

**Feature: NLP flight prompt parsing**

| Test | Protects |
|---|---|
| `test_parse_no_from` | Prompt with no "from" keyword |
| `test_parse_simple` | Basic "A to B" parsing |
| `test_parse_full` | Full prompt with aircraft, duration, time |
| `test_parse_duration` | Duration keyword extraction (short/long/haul) |
| `test_parse_country_as_region` | Country name → `Region` constraint |
| `test_parse_us_nickname_as_region` | "The States" → Region(US) |
| `test_parse_abbreviation_as_region` | "UK" abbreviation → Region(UK) |
| `test_parse_from_uk_only` | Single endpoint with direction |
| `test_parse_article_stripped` | "the " prefix stripped from city/region names |
| `test_parse_city_maps_to_nearcity` | City name → `NearCity` constraint |
| `test_parse_london_uk_to_region` | "London UK" → Region(UK), not NearCity |
| `test_parse_london_to_italy` | "London to Italy" → both endpoints correct |
| `test_parse_rome_italy_as_nearcity` | "Rome Italy" → NearCity(Rome), not KRMG |
| `test_parse_rome_comma_italy_as_nearcity` | "Rome, Italy" comma form |
| `test_parse_paris_france_as_nearcity` | "Paris France" city+country form |
| `test_parse_icao_still_icao` | 4-letter ICAO not mis-parsed as city |
| `test_parse_f70_to_alaska` | Short ICAO-like codes (< 4 chars) → AirportName |
| `test_parse_nairobi_to_lamu` | "Lamu" → NearCity (not ICAO "LAMU") |
| `test_parse_nairobi_to_mombasa` | African city name → NearCity |
| `test_parse_tokyo_to_bangkok` | Asian city name resolution |
| `test_parse_icao_still_works_after_reorder` | ICAO not affected by NLP rule reordering |
| `test_parse_washington_resolves_to_wa_state` | "Washington" → WA state, not DC |
| `test_parse_washington_state_explicit` | "Washington State" explicit form |
| `test_parse_washington_dc_still_works` | "Washington DC" → NearCity(DC) |
| `test_parse_civilian_airliner_with_landing` | "landing" in prompt doesn't break aircraft parse |
| `test_parse_time_and_weather` | `time: Dawn`, `weather: Storm` extracted |
| `test_parse_thunderstorm` | "thunderstorm" → WeatherKeyword::Storm |
| `test_parse_vfr_ifr` | "VFR" → Clear, "IFR" → Fog |

### `src/geo/mod.rs` — 2 unit tests

**Feature: RegionIndex bounding-box lookups**

| Test | Protects |
|---|---|
| `test_region_bounds` | Region bounds contain known airports |
| `test_region_search` | Fuzzy search returns correct region |

### `src/geo/data.rs` — 1 unit test

**Feature: regions.json load**

| Test | Protects |
|---|---|
| `test_regions_load` | All 154 regions parse without error |

### `src/parser.rs` — 3 unit tests

**Feature: Scenery pack name classifier (heuristics)**

| Test | Protects |
|---|---|
| `test_classify_airport` | Airport name pattern → Airport category |
| `test_classify_mesh` | Mesh/terrain name pattern |
| `test_classify_library` | Library name pattern |

### `tests/lfpg_test.rs` — 2 integration tests

**Feature: LFPG (Paris CDG) scoring edge case**

| Test | Protects |
|---|---|
| `test_lfpg_score` | LFPG airport pack scores correctly |
| `test_lfpg_sort_position` | LFPG sorts to correct position relative to mesh |

### `tests/ordering_guardrails.rs` — 5 integration tests

**Feature: Scenery ordering score guarantees**

| Test | Protects |
|---|---|
| `test_airport_above_mesh` | Airport score < mesh score |
| `test_overlay_above_mesh` | Overlay score < mesh score |
| `test_library_between_airport_and_mesh` | Library score in correct tier |
| `test_autoortho_at_bottom` | AutoOrtho at lowest priority |
| `test_global_airports_score` | Global airports score in correct tier |

---

## x-adox-core

### `src/flight_gen.rs` — 7 unit tests

**Feature: Flight generation internals**

| Test | Protects |
|---|---|
| `test_british_isles_matching` | "British Isles" region alias resolution |
| `test_airport_coords_for_poi_fetch` | Coordinate extraction with fallback |
| `test_jet_speed_estimate` | Jet speed heuristic for duration math |
| `test_bush_speed_override` | Bush flight speed override |
| `test_region_selection_by_bounds` | Airport filtered to region bounding box |
| `test_simbrief_url_orig_dest_type` | SimBrief URL format for ICAO/NearCity endpoints |
| `test_load_flight_context_from_json` | Flight context JSON load |

### `tests/flight_gen_test.rs` — 17 integration tests

**Feature: End-to-end flight generation correctness**

| Test | Protects |
|---|---|
| `test_generate_flight_simple` | Basic two-airport generation |
| `test_generate_flight_with_aircraft` | Aircraft tag filtering |
| `test_icao_pair_generation` | Explicit ICAO → ICAO routing |
| `test_short_flight_constraint` | `short` keyword → ≤ 200nm |
| `test_long_flight_constraint` | `long` keyword → ≥ 800nm |
| `test_surface_soft_preference` | `grass` keyword → soft surface preferred |
| `test_seaplane_base_only` | `floatplane` keyword → seaplane base only |
| `test_weather_confirmed_false_on_no_metar` | `weather_confirmed=false` when no METAR data |
| `test_export_fms11` | FMS 11 export format correctness |
| `test_export_fms12` | FMS 12 export format correctness |
| `test_export_lnmpln` | LNM `.lnmpln` export format correctness |
| `test_simbrief_url` | SimBrief URL construction |
| `test_pool_merge_pack_overrides_base` | Pack airport data overrides base layer |
| `test_pool_merge_keeps_longer_runway` | Longer runway wins on merge |
| `test_pool_deduplication` | Same ICAO from two sources → one entry |
| `test_pool_base_only_fast_path` | Empty packs → no BTreeMap allocation |
| `test_pool_new_airports_added_from_pack` | Pack-only airports appear in pool |

### `tests/flight_gen_robustness.rs` — 10 integration tests

**Feature: Region-level flight generation correctness**

| Test | Protects |
|---|---|
| `test_region_nlp_parsing` | Every region name in regions.json resolves via NLP |
| `test_all_regions_flight_generation` | Every seeded country generates a valid flight |
| `test_glider_short_keyword_constrains_range` | Short keyword caps GA distance |
| `test_italy_accuracy` | Italy region uses LI prefix, not French (LF) airports |
| `test_rome_italy_resolves_to_italy_not_usa` | "Rome Italy" → LIRF not KRMG |
| `test_search_accuracy_london_uk` | "London UK" → EG* not CYXU (Ontario) |
| `test_search_accuracy_london_england` | "London England" → EGLC by name scoring |
| `test_england_to_ukraine` | Cross-region generation, both endpoints correct |
| `test_california_to_england_stays_in_england` | England bounds exclude Scotland |
| `test_nairobi_to_lamu` | African city pair via seed airports |

### `tests/flight_gen_stress.rs` — 4 tests (some `#[ignore]`)

**Feature: Fuzzing / stress testing**

| Test | Protects |
|---|---|
| `stress_random_prompts` | Random prompt generation doesn't panic |
| `stress_regenerate_same_prompt` | Repeated generation of same prompt stable |
| `stress_missing_runway_data_explicit_dest` | Missing runway data doesn't panic |
| *(ignored by default)* | Long-running stress; run with `--include-ignored` |

### `tests/regression_validator.rs` — 21 tests

**Feature: Scenery order validation**

| Test | Protects |
|---|---|
| `test_simheaven_below_global_airports_is_ok` | Valid order produces no issues |
| `test_simheaven_above_global_airports_is_critical` | SimHeaven above GA → Critical |
| `test_multiple_simheaven_above_global_airports` | Multiple violations detected |
| `test_x_world_name_also_triggers_simheaven_check` | X-World name variant triggers same check |
| `test_no_global_airports_means_no_simheaven_issue` | No GA virtual pack → no false positive |
| `test_mesh_above_airport_triggers_warning` | Mesh above airport → Warning |
| `test_ortho_above_airport_triggers_warning` | Ortho above airport → Warning |
| `test_mesh_below_all_overlays_is_ok` | Clean order → no issues |
| `test_library_between_mesh_and_overlay_no_warning` | Libraries are position-independent |
| `test_mesh_above_regional_fluff_triggers_warning` | Regional mesh ordering |
| `test_mesh_above_auto_ortho_overlay_triggers_warning` | AutoOrtho overlay ordering |
| `test_mesh_fully_shadowed_triggers_warning` | Fully shadowed mesh detected |
| `test_mesh_partially_overlapping_no_shadow` | Partial overlap → no false positive |
| `test_non_mesh_tiles_not_checked_for_shadowing` | Airport tiles not checked |
| `test_disabled_mesh_not_checked_for_shadowing` | Disabled packs excluded from validation |
| `test_empty_tiles_no_shadowing` | Empty tile list → no false positive |
| `test_multiple_validation_issues_simultaneously` | Multiple issues detected in one pass |
| `test_clean_ordering_produces_no_issues` | Fully correct order → zero issues |
| `test_specific_mesh_not_checked_for_shadowing` | Mesh-only packs exempt |
| `test_specific_mesh_above_overlay_no_warning` | Specific mesh exempt from overlay check |
| `test_empty_pack_list_no_issues` | Empty list → no crash |

### `tests/regression_classification.rs` — 9 tests

**Feature: Heuristic scenery classification**

| Test | Protects |
|---|---|
| `test_community_libraries_classified_by_name` | Library name patterns → Library category |
| `test_landmarks_without_hyphen` | Landmark without hyphen → Landmark category |
| `test_landmarks_with_hyphen_still_works` | Hyphenated landmark name |
| `test_library_txt_prevents_airport_promotion` | `library.txt` → Library, not Airport |
| `test_orbx_airport_mesh_not_classified_as_generic_mesh` | ORBX airport-mesh classified correctly |
| `test_orbx_d_mesh_still_classified_as_mesh` | ORBX pure mesh correctly classified |
| `test_icao_companion_packs_classified_as_specific_mesh` | Airport companion pack classification |
| `test_flytampa_mesh_still_classified_as_mesh` | FlyTampa mesh classification |
| `test_orthobase_with_airports_stays_orthobase` | OrthoBase + airports stays OrthoBase |

### `tests/regression_score_modifiers.rs` — 16 tests

**Feature: Score modifier system (manual priority overrides)**

Covers: pin persistence, score bump/penalty application, modifier serialization/deserialization.

### `tests/regression_icao.rs` — 15 tests

**Feature: ICAO airport extraction from scenery packs**

Covers: multi-airport packs, ICAO extraction from names, pack-to-airport mapping.

### `tests/regression_simheaven.rs` — 3 tests

**Feature: SimHeaven layer ordering specifics**

| Test | Protects |
|---|---|
| `test_simheaven_layer_numerical_sorting` | SimHeaven HD1/HD2/HD3 sort order |
| `test_simheaven_continent_grouping` | Continent grouping stays intact |
| `test_simheaven_vegetation_library_position` | Vegetation library position |

### `tests/regression_toggle.rs` — 1 test

**Feature: Enable/disable scenery pack persistence**

| Test | Protects |
|---|---|
| `test_scenery_toggle_persistence` | Toggle written and re-read from INI |

### `tests/regression_pinning.rs` — 2 tests

**Feature: Manual pin system**

Covers: pin survives Smart Sort, pin cleared on user request.

### `tests/regression_basket.rs` — 2 tests

**Feature: Scenery basket bulk operations**

Covers: basket enable/disable, auto-pin on basket operation.

### `tests/regression_profiles.rs` — 2 tests

**Feature: Profile switching**

Covers: profile creation, scenery state isolated per profile.

### `tests/regression_deletion.rs` — 3 tests

**Feature: Permanent scenery deletion**

Covers: deletion removes from INI, deletion removes folder, disabled-pack deletion.

### `tests/regression_aircraft.rs` — 3 tests

**Feature: Aircraft enable/disable**

Covers: aircraft disable moves to `(Disabled)` folder, re-enable restores.

### `tests/regression_hashing_migration.rs` — 3 tests

**Feature: Install-path hash stability across restarts**

Covers: FNV-1a hash determinism, legacy hash migration, cross-device migration fallback.

### `tests/regression_regions.rs` — 2 tests

**Feature: Geographic region assignment to scenery packs**

Covers: region assigned from airport coords, packs without airports get no region.

### `tests/regression_path_normalization.rs` — 1 test

**Feature: Install path normalization**

Covers: symlinks resolved, trailing slashes stripped, case variants normalized.

### `tests/regression_literal_paths.rs` — 1 test

**Feature: INI round-trip with literal (non-normalized) paths**

Covers: `raw_path` written verbatim, no path normalization on write.

### `tests/integration_tests.rs` — 5 tests

**Feature: Full scenery manager lifecycle**

Covers: load from INI, Smart Sort, save, reload cycle.

### `tests/e2e_tests.rs` — 1 test

**Feature: End-to-end scenery workflow**

Covers: INI read → sort → write → re-read parity.

### `tests/pin_survival.rs` — 3 tests

**Feature: Pin persistence across Smart Sort cycles**

Covers: manually moved pack stays pinned after sort, multi-pack pin survival.

### `tests/laminar_suppress_test.rs` — 3 tests

**Feature: Laminar default aircraft suppression**

Covers: default aircraft not shown in main list, can be un-suppressed.

### `tests/profile_persistence_test.rs` — 1 test

**Feature: Profile config file round-trip**

Covers: profile saved to disk, reloaded correctly.

### `tests/content_aware_classification.rs` — 4 tests

**Feature: Content-aware classification (post-discovery promotion)**

Covers: `library.txt` promotes to Library, `apt.dat` promotes to Airport, DSF tiles → Mesh.

### `tests/scenery_robustness.rs` — 3 tests

**Feature: Scenery manager robustness**

Covers: missing INI file, corrupt INI, empty pack list.

### `src/scenery/mod.rs` — 9 unit tests

**Feature: INI parsing and pack reconciliation**

Covers: INI parse, pack reconciliation, disabled pack detection, sort stability.

### `src/scenery/sorter.rs` — 6 unit tests

**Feature: Smart sort algorithm**

Covers: stable sort, pin respect, category ordering, equal-score preservation.

### `src/lib.rs` — 5 unit tests

**Feature: Core utilities**

Covers: FNV-1a hash stability, config root detection, path normalization.

### `src/apt_dat.rs` — 1 unit test

**Feature: apt.dat parser**

Covers: basic airport row parse.

### `src/logbook.rs` — 2 unit tests

**Feature: Pilot.txt logbook parser**

Covers: standard entry parse, malformed entry handling.

### `src/weather.rs` — 1 unit test

**Feature: METAR determination logic**

Covers: Rain/Storm/Snow/Fog/Gusty/Calm/Clear determination from MetarRecord.

---

## Coverage Gaps

The following areas have **no dedicated tests** and rely on integration tests for indirect coverage:

| Area | Risk | Notes |
|---|---|---|
| `cache.rs` serialization format | Low | Covered indirectly by scenery lifecycle tests |
| `map.rs` tile manager | Low | GUI-only, visual testing only |
| `management.rs` file moves | Medium | Covered via regression_aircraft + regression_toggle |
| NLP `speed_kts` / `priority` fields | Low | Covered by flight_prompt unit tests for adjacent features |
| Airport pool merge (fast path vs slow path) | Medium | See `regression_airport_pool.rs` for dedicated coverage |
| `RegionIndex` HashMap index | Low | Covered by geo/mod.rs unit tests + robustness tests |

---

## Running the Tests

```bash
# Full suite
cargo test

# Single crate
cargo test -p x-adox-core
cargo test -p x-adox-bitnet

# Specific test file
cargo test -p x-adox-core --test flight_gen_robustness
cargo test -p x-adox-core --test regression_validator

# Single test by name
cargo test -p x-adox-core test_england_to_ukraine

# Include ignored (stress) tests
cargo test -p x-adox-core --test flight_gen_stress -- --include-ignored --nocapture

# Reproducible stress test failure
STRESS_SEED=12345 cargo test -p x-adox-core --test flight_gen_stress -- --include-ignored --nocapture
```

---

*Last updated: February 2026. Update this file when adding or removing tests.*
