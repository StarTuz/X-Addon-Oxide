use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};
use std::path::{Path, PathBuf};
use x_adox_bitnet::HeuristicsConfig;
use x_adox_core::apt_dat::Airport;
use x_adox_core::discovery::DiscoveredAddon;
use x_adox_core::flight_gen::{self, FlightContext, FlightPlan, FLIGHT_CONTEXT_CACHE_DIR};
use x_adox_core::get_config_root;

#[derive(Debug, Clone)]
pub struct FlightGenState {
    pub input_value: String,
    pub history: Vec<ChatMessage>,
    pub current_plan: Option<FlightPlan>,
    pub status_message: Option<String>,
    /// Base airport layer (Option B): loaded from X-Plane Resources/Global Scenery when root is set.
    pub base_airports: Option<Vec<Airport>>,
    /// When true, main should run enhanced context fetch if the setting is on (so history appears without clicking "Fetch context").
    pub pending_auto_fetch: bool,
    /// Tree view: Origin section expanded in History & context.
    pub origin_context_expanded: bool,
    /// Tree view: Destination section expanded in History & context.
    pub dest_context_expanded: bool,
    /// As-of-now weather (decoded METAR) for departure airport. Set when context/weather is fetched.
    pub origin_weather: Option<String>,
    /// As-of-now weather (decoded METAR) for destination airport.
    pub dest_weather: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub sender: String,
    pub text: String,
    pub is_user: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    Submit,
    /// Regenerate a new plan from the same prompt (same request, new random outcome).
    Regenerate,
    ExportFms11,
    ExportFms12,
    ExportLnm,
    ExportSimbrief,
    /// Remember this flight so the same origin/dest pair is preferred next time for the same region pair.
    RememberThisFlight,
    /// Prefer this origin airport for its region (e.g. "Prefer HKJK for Kenya").
    PreferThisOrigin,
    /// Prefer this destination airport for its region.
    PreferThisDestination,
    /// Fetch context from API (Phase 2b); handled in main with Task::run, then ContextFetched.
    FetchContext,
    /// Toggle Origin tree section expanded/collapsed in History & context.
    ToggleOriginContext,
    /// Toggle Destination tree section expanded/collapsed in History & context.
    ToggleDestinationContext,
    /// Weather fetched for current plan (origin, dest). Called from main after FlightWeatherFetched.
    WeatherFetched(Option<String>, Option<String>),
}

impl Default for FlightGenState {
    fn default() -> Self {
        Self {
            input_value: String::new(),
            history: vec![ChatMessage {
                sender: "System".to_string(),
                text: "Welcome to the Flight Generator. Ask for a flight! e.g., 'Flight from EGLL to LFPG using Cessna'".to_string(),
                is_user: false,
            }],
            current_plan: None,
            status_message: None,
            base_airports: None,
            pending_auto_fetch: false,
            origin_context_expanded: true,
            dest_context_expanded: true,
            origin_weather: None,
            dest_weather: None,
        }
    }
}

/// True when the plan has no history content to show (no snippets, no POIs).
pub fn plan_context_is_empty(plan: &FlightPlan) -> bool {
    match &plan.context {
        None => true,
        Some(ctx) => {
            ctx.origin.snippet.is_empty()
                && ctx.origin.points_nearby.is_empty()
                && ctx.destination.snippet.is_empty()
                && ctx.destination.points_nearby.is_empty()
        }
    }
}

use x_adox_core::scenery::SceneryPack;

impl FlightGenState {
    pub fn update(
        &mut self,
        message: Message,
        packs: &[SceneryPack],
        aircraft_list: &[DiscoveredAddon],
        xplane_root: Option<&Path>,
        prefs: Option<&HeuristicsConfig>,
    ) {
        match message {
            Message::InputChanged(val) => {
                self.input_value = val;
            }
            Message::Submit => {
                if self.input_value.trim().is_empty() {
                    return;
                }
                // Option B: load base airport layer once when we have X-Plane root
                if xplane_root.is_some() && self.base_airports.is_none() {
                    self.base_airports = Some(flight_gen::load_base_airports(xplane_root.unwrap()));
                }

                let prompt = self.input_value.clone();
                self.history.push(ChatMessage {
                    sender: "User".to_string(),
                    text: prompt.clone(),
                    is_user: true,
                });
                self.input_value.clear();

                let base = self.base_airports.as_deref();
                match flight_gen::generate_flight(packs, aircraft_list, &prompt, base, prefs) {
                    Ok(mut plan) => {
                        if let Some(ctx) = load_flight_context_for_plan(
                            get_config_root().as_path(),
                            &plan.origin,
                            &plan.destination,
                        ) {
                            plan.context = Some(ctx);
                        }
                        let response = format!(
                            "Generated Flight:\nOrigin: {} ({})\nDestination: {} ({})\nAircraft: {}\nDistance: {} nm\nDuration: {} mins",
                            plan.origin.id, plan.origin.name,
                            plan.destination.id, plan.destination.name,
                            plan.aircraft.name,
                            plan.distance_nm,
                            plan.duration_minutes
                        );
                        self.history.push(ChatMessage {
                            sender: "System".to_string(),
                            text: response,
                            is_user: false,
                        });
                        self.current_plan = Some(plan);
                        self.origin_context_expanded = true;
                        self.dest_context_expanded = true;
                        self.pending_auto_fetch =
                            plan_context_is_empty(self.current_plan.as_ref().unwrap());
                        self.status_message = Some("Flight generated successfully.".to_string());
                    }
                    Err(e) => {
                        let mut text = format!("Error: {}", e);
                        // Option A: suggest adding Global Airports when not in list
                        let has_global = packs.iter().any(|p| {
                            p.name == "Global Airports"
                                || p.name == "*GLOBAL_AIRPORTS*"
                                || p.path
                                    .to_string_lossy()
                                    .to_lowercase()
                                    .contains("global airports")
                        });
                        if !has_global
                            && (e.contains("No suitable departure")
                                || e.contains("No suitable destination"))
                        {
                            text.push_str(
                                "\nTip: Add Global Airports in Scenery for more options.",
                            );
                        }
                        self.history.push(ChatMessage {
                            sender: "System".to_string(),
                            text: text.clone(),
                            is_user: false,
                        });
                        self.status_message = Some(text);
                    }
                }
            }
            Message::Regenerate => {
                let prompt = self
                    .history
                    .iter()
                    .rev()
                    .find(|m| m.is_user)
                    .map(|m| m.text.clone());
                if let Some(prompt) = prompt {
                    let base = self.base_airports.as_deref();
                    match flight_gen::generate_flight(packs, aircraft_list, &prompt, base, prefs) {
                        Ok(mut plan) => {
                            if let Some(ctx) = load_flight_context_for_plan(
                                get_config_root().as_path(),
                                &plan.origin,
                                &plan.destination,
                            ) {
                                plan.context = Some(ctx);
                            }
                            let response = format!(
                                "Generated Flight:\nOrigin: {} ({})\nDestination: {} ({})\nAircraft: {}\nDistance: {} nm\nDuration: {} mins",
                                plan.origin.id, plan.origin.name,
                                plan.destination.id, plan.destination.name,
                                plan.aircraft.name,
                                plan.distance_nm,
                                plan.duration_minutes
                            );
                            self.history.push(ChatMessage {
                                sender: "System".to_string(),
                                text: response,
                                is_user: false,
                            });
                            self.current_plan = Some(plan);
                            self.origin_context_expanded = true;
                            self.dest_context_expanded = true;
                            self.pending_auto_fetch =
                                plan_context_is_empty(self.current_plan.as_ref().unwrap());
                            self.status_message = Some("Flight regenerated.".to_string());
                        }
                        Err(e) => {
                            let mut text = format!("Error: {}", e);
                            let has_global = packs.iter().any(|p| {
                                p.name == "Global Airports"
                                    || p.name == "*GLOBAL_AIRPORTS*"
                                    || p.path
                                        .to_string_lossy()
                                        .to_lowercase()
                                        .contains("global airports")
                            });
                            if !has_global
                                && (e.contains("No suitable departure")
                                    || e.contains("No suitable destination"))
                            {
                                text.push_str(
                                    "\nTip: Add Global Airports in Scenery for more options.",
                                );
                            }
                            self.history.push(ChatMessage {
                                sender: "System".to_string(),
                                text,
                                is_user: false,
                            });
                            // FIXED: Do NOT clear current_plan here.
                            // If we fail a regenerate, we should keep the previous plan's buttons
                            // so the user can still export it or try regenerating again.
                        }
                    }
                }
            }
            Message::ExportFms11 => {
                if let Some(plan) = &self.current_plan {
                    let _text = flight_gen::export_fms_11(plan);
                    // TODO: Save to file logic should happen here or via file picker
                    // For now just simulation
                    self.status_message = Some("Exported FMS 11 (simulated)".to_string());
                }
            }
            Message::ExportFms12 => {
                if let Some(plan) = &self.current_plan {
                    let _ = flight_gen::export_fms_12(plan);
                    self.status_message = Some("Exported FMS 12 (simulated)".to_string());
                }
            }
            Message::ExportLnm => {
                if let Some(plan) = &self.current_plan {
                    let _ = flight_gen::export_lnmpln(plan);
                    self.status_message = Some("Exported Little Navmap (simulated)".to_string());
                }
            }
            Message::ExportSimbrief => {
                if let Some(plan) = &self.current_plan {
                    let url = flight_gen::export_simbrief(plan);
                    // Open URL?
                    self.status_message = Some(format!("SimBrief URL: {}", url));
                }
            }
            // Handled in main (Task::run + FlightContextFetched).
            Message::FetchContext => {}
            Message::ToggleOriginContext => {
                self.origin_context_expanded = !self.origin_context_expanded;
            }
            Message::ToggleDestinationContext => {
                self.dest_context_expanded = !self.dest_context_expanded;
            }
            Message::WeatherFetched(origin, dest) => {
                self.origin_weather = origin;
                self.dest_weather = dest;
            }

            // Handled in main (mutate heuristics_model); no-op here.
            Message::RememberThisFlight
            | Message::PreferThisOrigin
            | Message::PreferThisDestination => {}
        }
    }

    /// Apply result of background fetch (Phase 2b). Call from main on Message::FlightContextFetched.
    pub fn apply_fetched_context(&mut self, result: Result<FlightContext, String>) {
        match result {
            Ok(ctx) => {
                let no_origin_landmarks = ctx.origin.points_nearby.is_empty();
                let no_dest_landmarks = ctx.destination.points_nearby.is_empty();
                if let Some(plan) = &mut self.current_plan {
                    plan.context = Some(ctx);
                }
                self.origin_context_expanded = true;
                self.dest_context_expanded = true;
                self.status_message = Some(if no_origin_landmarks && no_dest_landmarks {
                    "Context loaded. No landmarks found — try Fetch context again (check network)."
                        .to_string()
                } else if no_origin_landmarks || no_dest_landmarks {
                    "Context loaded. Some landmarks missing — try Fetch context again.".to_string()
                } else {
                    "Context loaded.".to_string()
                });
            }
            Err(e) => {
                self.status_message = Some(format!("Context fetch failed: {}", e));
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let chat_history = scrollable(
            column(self.history.iter().map(|msg| {
                container(column![
                    text(&msg.sender).size(12).style(move |_| if msg.is_user {
                        iced::widget::text::Style::default()
                    } else {
                        iced::widget::text::Style::default()
                    }),
                    text(&msg.text).size(16)
                ])
                .padding(10)
                .style(move |_| {
                    if msg.is_user {
                        container::Style {
                            background: Some(iced::Background::Color(iced::Color::from_rgb(
                                0.2, 0.2, 0.3,
                            ))),
                            ..Default::default()
                        }
                    } else {
                        container::Style {
                            background: Some(iced::Background::Color(iced::Color::from_rgb(
                                0.15, 0.15, 0.15,
                            ))),
                            ..Default::default()
                        }
                    }
                })
                .into()
            }))
            .spacing(10),
        )
        .height(Length::Fill)
        .width(Length::Fill);

        let input_area = row![
            text_input("Ask for a flight...", &self.input_value)
                .on_input(Message::InputChanged)
                .on_submit(Message::Submit)
                .padding(10),
            button("Send").on_press(Message::Submit).padding(10)
        ]
        .spacing(10);

        let mut controls = row![].spacing(10);

        // Always show Regenerate if we have at least one user prompt to regenerate from
        if self.history.iter().any(|m| m.is_user) {
            controls = controls.push(button("Regenerate").on_press(Message::Regenerate));
        }

        if let Some(plan) = &self.current_plan {
            controls = controls.push(button("FMS 11").on_press(Message::ExportFms11));
            controls = controls.push(button("FMS 12").on_press(Message::ExportFms12));
            controls = controls.push(button("LNM").on_press(Message::ExportLnm));
            controls = controls.push(button("SimBrief").on_press(Message::ExportSimbrief));

            if plan.origin_region_id.is_some() && plan.dest_region_id.is_some() {
                controls = controls
                    .push(button("Remember this flight").on_press(Message::RememberThisFlight));
            }
            if plan.origin_region_id.is_some() {
                controls =
                    controls.push(button("Prefer this origin").on_press(Message::PreferThisOrigin));
            }
            if plan.dest_region_id.is_some() {
                controls = controls.push(
                    button("Prefer this destination").on_press(Message::PreferThisDestination),
                );
            }
            controls = controls.push(button("Fetch context").on_press(Message::FetchContext));
        }

        let status_row = text(self.status_message.as_deref().unwrap_or("")).size(14);

        let history_block = self.view_history_and_context();

        column![
            chat_history,
            controls,
            history_block,
            status_row,
            input_area
        ]
        .spacing(20)
        .padding(20)
        .into()
    }

    /// "History & context" block: tree view with Origin and Destination sections; each has History, Landmarks, and Weather (as of now).
    fn view_history_and_context(&self) -> Element<'_, Message> {
        let Some(plan) = &self.current_plan else {
            return column![].into();
        };

        let header = text("History & context").size(14);

        let (origin_inner, dest_inner) = match &plan.context {
            Some(ctx) => (self.origin_tree_content(ctx), self.dest_tree_content(ctx)),
            None => (
                column![
                    text("No context loaded. Click \"Fetch context\" to load history and weather.")
                        .size(11)
                        .color(iced::Color::from_rgb(0.55, 0.55, 0.6)),
                    text("Weather (as of now)")
                        .size(11)
                        .color(iced::Color::from_rgb(0.7, 0.7, 0.75)),
                    text(self.origin_weather.as_deref().unwrap_or("—")).size(11),
                ]
                .spacing(6)
                .into(),
                column![
                    text("No context loaded. Click \"Fetch context\" to load history and weather.")
                        .size(11)
                        .color(iced::Color::from_rgb(0.55, 0.55, 0.6)),
                    text("Weather (as of now)")
                        .size(11)
                        .color(iced::Color::from_rgb(0.7, 0.7, 0.75)),
                    text(self.dest_weather.as_deref().unwrap_or("—")).size(11),
                ]
                .spacing(6)
                .into(),
            ),
        };

        let origin_header = row![
            button(
                text(if self.origin_context_expanded {
                    "▾ "
                } else {
                    "▸ "
                })
                .size(12)
            )
            .on_press(Message::ToggleOriginContext)
            .padding(2),
            button(text(format!("Origin: {} ({})", plan.origin.name, plan.origin.id)).size(12))
                .on_press(Message::ToggleOriginContext)
                .padding(2),
        ]
        .spacing(4);

        let dest_header = row![
            button(
                text(if self.dest_context_expanded {
                    "▾ "
                } else {
                    "▸ "
                })
                .size(12)
            )
            .on_press(Message::ToggleDestinationContext)
            .padding(2),
            button(
                text(format!(
                    "Destination: {} ({})",
                    plan.destination.name, plan.destination.id
                ))
                .size(12)
            )
            .on_press(Message::ToggleDestinationContext)
            .padding(2),
        ]
        .spacing(4);

        let origin_section = column![
            origin_header,
            if self.origin_context_expanded {
                column![origin_inner].padding(20)
            } else {
                column![]
            },
        ]
        .spacing(4);

        let dest_section = column![
            dest_header,
            if self.dest_context_expanded {
                column![dest_inner].padding(20)
            } else {
                column![]
            },
        ]
        .spacing(4);

        let enroute_note = text("En route: Check NOTAMs and winds aloft for your route.")
            .size(11)
            .color(iced::Color::from_rgb(0.6, 0.6, 0.65));

        let content = scrollable(column![origin_section, dest_section, enroute_note,].spacing(12))
            .height(Length::Fill); // Allow it to fill the container, which we will constrain

        container(column![header, content].spacing(10).padding(10))
            .height(Length::Fixed(300.0)) // Fixed height to prevent pushing other elements off
            .style(|_| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.12, 0.12, 0.14,
                ))),
                border: iced::Border {
                    color: iced::Color::from_rgb(0.3, 0.3, 0.35),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn origin_tree_content<'a>(&'a self, ctx: &'a FlightContext) -> Element<'a, Message> {
        let history_label = text("History")
            .size(11)
            .color(iced::Color::from_rgb(0.7, 0.7, 0.75));
        let history_body = if ctx.origin.snippet.is_empty() {
            text("No Wikipedia summary for this airport. Enable \"Enhanced history from Wikipedia\" in Settings and click \"Fetch context\" to load one.")
                .size(11)
                .color(iced::Color::from_rgb(0.55, 0.55, 0.6))
        } else {
            text(&ctx.origin.snippet).size(12)
        };
        let landmarks_label = text("Surrounding Landmarks (within 10 nm)")
            .size(11)
            .color(iced::Color::from_rgb(0.7, 0.7, 0.75));
        let landmarks_attribution = text("From Wikipedia and Wikidata (geo-tagged). Wrong or missing? Edit on Wikipedia or add to the overlay (see docs).").size(10).color(iced::Color::from_rgb(0.5, 0.5, 0.55));
        let landmarks_list: Vec<Element<_>> = if ctx.origin.points_nearby.is_empty() {
            vec![text("No landmarks in range. Click \"Fetch context\" to load from Wikipedia and Wikidata.").size(11).color(iced::Color::from_rgb(0.55, 0.55, 0.6)).into()]
        } else {
            ctx.origin
                .points_nearby
                .iter()
                .map(|poi| {
                    let line = if let Some(d) = poi.distance_nm {
                        format!("• {} — {} ({:.1} nm)", poi.name, poi.snippet, d)
                    } else {
                        format!("• {} — {}", poi.name, poi.snippet)
                    };
                    text(line)
                        .size(11)
                        .color(iced::Color::from_rgb(0.85, 0.85, 0.85))
                        .into()
                })
                .collect()
        };
        let weather_label = text("Weather (as of now)")
            .size(11)
            .color(iced::Color::from_rgb(0.7, 0.7, 0.75));
        let weather_body = text(
            self.origin_weather
                .as_deref()
                .unwrap_or("Click \"Fetch context\" to load METAR."),
        )
        .size(11);
        column![
            column![history_label, history_body].spacing(4),
            column![
                landmarks_label,
                landmarks_attribution,
                column(landmarks_list).spacing(4)
            ]
            .spacing(4),
            column![weather_label, weather_body].spacing(4),
        ]
        .spacing(10)
        .into()
    }

    fn dest_tree_content<'a>(&'a self, ctx: &'a FlightContext) -> Element<'a, Message> {
        let history_label = text("History")
            .size(11)
            .color(iced::Color::from_rgb(0.7, 0.7, 0.75));
        let history_body = if ctx.destination.snippet.is_empty() {
            text("No Wikipedia summary for this airport. Enable \"Enhanced history from Wikipedia\" in Settings and click \"Fetch context\" to load one.")
                .size(11)
                .color(iced::Color::from_rgb(0.55, 0.55, 0.6))
        } else {
            text(&ctx.destination.snippet).size(12)
        };
        let landmarks_label = text("Surrounding Landmarks (within 10 nm)")
            .size(11)
            .color(iced::Color::from_rgb(0.7, 0.7, 0.75));
        let landmarks_attribution = text("From Wikipedia and Wikidata (geo-tagged). Wrong or missing? Edit on Wikipedia or add to the overlay (see docs).").size(10).color(iced::Color::from_rgb(0.5, 0.5, 0.55));
        let landmarks_list: Vec<Element<_>> = if ctx.destination.points_nearby.is_empty() {
            vec![text("No landmarks in range. Click \"Fetch context\" to load from Wikipedia and Wikidata.").size(11).color(iced::Color::from_rgb(0.55, 0.55, 0.6)).into()]
        } else {
            ctx.destination
                .points_nearby
                .iter()
                .map(|poi| {
                    let line = if let Some(d) = poi.distance_nm {
                        format!("• {} — {} ({:.1} nm)", poi.name, poi.snippet, d)
                    } else {
                        format!("• {} — {}", poi.name, poi.snippet)
                    };
                    text(line)
                        .size(11)
                        .color(iced::Color::from_rgb(0.85, 0.85, 0.85))
                        .into()
                })
                .collect()
        };
        let weather_label = text("Weather (as of now)")
            .size(11)
            .color(iced::Color::from_rgb(0.7, 0.7, 0.75));
        let weather_body = text(
            self.dest_weather
                .as_deref()
                .unwrap_or("Click \"Fetch context\" to load METAR."),
        )
        .size(11);
        column![
            column![history_label, history_body].spacing(4),
            column![
                landmarks_label,
                landmarks_attribution,
                column(landmarks_list).spacing(4)
            ]
            .spacing(4),
            column![weather_label, weather_body].spacing(4),
        ]
        .spacing(10)
        .into()
    }
}

/// Loads flight context from bundled + config + cache (Option B). No network. Use after generate or for "Load history".
pub fn load_flight_context_for_plan(
    config_root: &Path,
    origin: &Airport,
    destination: &Airport,
) -> Option<FlightContext> {
    let bundled = flight_gen::get_bundled_flight_context();
    let config_path = config_root.join("flight_context.json");
    let cache_dir = config_root.join(FLIGHT_CONTEXT_CACHE_DIR);
    flight_gen::load_flight_context_with_bundled(
        &bundled,
        &config_path,
        &cache_dir,
        origin,
        destination,
        None,
        None,
    )
}

const WIKIPEDIA_SUMMARY_URL: &str = "https://en.wikipedia.org/api/rest_v1/page/summary";

/// Wikipedia API geosearch: pages near lat/lon. Max radius 10 km (~5.4 nm); results merged with overlay and filtered to 10 nm in core.
const WIKIPEDIA_GEOSEARCH_URL: &str = "https://en.wikipedia.org/w/api.php";

/// Geosearch cache TTL: 7 days (same coords reuse cached POIs).
const POIS_NEAR_CACHE_TTL_SECS: u64 = 7 * 24 * 3600;

/// Subdir under flight_context_cache for geosearch POI cache (one file per lat/lon bucket).
const POIS_NEAR_CACHE_SUBDIR: &str = "pois_near";

/// Wikidata Query Service: semantic POIs (stadium, pier, museum, etc.) near a point. Cached under pois_near_wikidata.
const WIKIDATA_SPARQL_URL: &str = "https://query.wikidata.org/sparql";
const POIS_NEAR_WIKIDATA_CACHE_SUBDIR: &str = "pois_near_wikidata";

/// Subdir for caching Wikipedia extract per page title (for POI descriptions).
const POI_EXTRACT_CACHE_SUBDIR: &str = "poi_extract";

/// Extract cache TTL: 7 days.
const POI_EXTRACT_CACHE_TTL_SECS: u64 = 7 * 24 * 3600;

/// Max snippet length for POI descriptions (travelogue-style).
const POI_SNIPPET_MAX_LEN: usize = 280;

/// Hardcoded fallback when primary Wikipedia request fails (e.g. rate limit, network). Proxy wraps same API.
const WIKIPEDIA_FALLBACK_PROXY_BASE: &str = "https://api.allorigins.win/raw?url=";

/// Aviation Weather Center METAR (decoded). No API key required.
const AWC_METAR_URL: &str = "https://aviationweather.gov/api/data/metar";

/// Timeout for POI fetch requests (Wikipedia, Wikidata). Prevents silent failure on slow networks.
const POI_FETCH_TIMEOUT_SECS: u64 = 30;

fn poi_fetch_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(POI_FETCH_TIMEOUT_SECS))
        .build()
}

/// Fetches decoded METAR for origin and destination ICAO. Returns (origin_text, dest_text). Used for "Weather (as of now)" in History & context.
pub fn fetch_weather_for_plan(
    origin_icao: &str,
    dest_icao: &str,
) -> (Option<String>, Option<String>) {
    let mut requested_ids = vec![
        origin_icao.trim().to_uppercase(),
        dest_icao.trim().to_uppercase(),
    ];

    // Add K-prefixed versions for 3-char US airports (e.g. F70 -> KF70)
    let origin_icao_up = origin_icao.trim().to_uppercase();
    let dest_icao_up = dest_icao.trim().to_uppercase();

    if origin_icao_up.len() == 3 {
        requested_ids.push(format!("K{}", origin_icao_up));
    }
    if dest_icao_up.len() == 3 {
        requested_ids.push(format!("K{}", dest_icao_up));
    }

    let ids = requested_ids.join(",");
    let url = format!(
        "{}?ids={}&format=decoded",
        AWC_METAR_URL,
        urlencoding::encode(&ids)
    );
    let agent = ureq::Agent::new();
    let resp = match agent
        .get(&url)
        .set("User-Agent", "X-Addon-Oxide/1.0 (flight context)")
        .call()
    {
        Ok(r) => r,
        Err(_) => return (None, None),
    };
    let body = match resp.into_string() {
        Ok(s) => s,
        Err(_) => return (None, None),
    };

    // Parse response into a map of ICAO -> Raw Text
    let mut metar_map = std::collections::HashMap::new();
    for s in body.split("METAR for ") {
        let t = s.trim();
        if t.is_empty() {
            continue;
        }
        // Extract ICAO from the start (e.g. "KDLS ...")
        if let Some(icao) = t.split_whitespace().next() {
            // The API sometimes returns "KDLS (The Dalles..." so we just take the first token
            let key = icao
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_uppercase();
            metar_map.insert(key, format!("METAR for {}", t));
        }
    }

    let get_with_fallback = |icao: &str| {
        let icao = icao.to_uppercase();
        if let Some(m) = metar_map.get(&icao) {
            return Some(m.clone());
        }
        if icao.len() == 3 {
            let k_icao = format!("K{}", icao);
            if let Some(m) = metar_map.get(&k_icao) {
                return Some(m.clone());
            }
        }
        None
    };

    let origin = get_with_fallback(&origin_icao_up);
    let dest = get_with_fallback(&dest_icao_up);

    (origin, dest)
}

fn safe_title_for_filename(title: &str) -> String {
    let s: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    s.trim()
        .chars()
        .take(80)
        .collect::<String>()
        .replace(' ', "_")
}

/// Fetches or loads from cache the Wikipedia extract for a page title. Used to enrich POI snippets.
fn get_extract_for_wikipedia_title(
    title: &str,
    cache_dir: &Path,
    fallback_proxy: Option<&str>,
) -> Option<String> {
    use std::time::{Duration, SystemTime};
    let sub = cache_dir.join(POI_EXTRACT_CACHE_SUBDIR);
    let _ = std::fs::create_dir_all(&sub);
    let key = safe_title_for_filename(title);
    let path = sub.join(format!("{}.txt", key));
    if let Ok(meta) = std::fs::metadata(&path) {
        if let Ok(mtime) = meta.modified() {
            let age = SystemTime::now()
                .duration_since(mtime)
                .unwrap_or(Duration::MAX);
            if age.as_secs() < POI_EXTRACT_CACHE_TTL_SECS {
                if let Ok(s) = std::fs::read_to_string(&path) {
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }
    }
    let primary_url = format!(
        "{}/{}",
        WIKIPEDIA_SUMMARY_URL.trim_end_matches('/'),
        urlencoding::encode(title)
    );
    let body = fetch_wikipedia_summary_body(&primary_url, fallback_proxy)?;
    let v: serde_json::Value = serde_json::from_str(&body).ok()?;
    let extract = v.get("extract")?.as_str()?.to_string();
    if extract.is_empty() {
        return None;
    }
    let truncated = if extract.len() > POI_SNIPPET_MAX_LEN {
        format!(
            "{}…",
            extract
                .chars()
                .take(POI_SNIPPET_MAX_LEN)
                .collect::<String>()
                .trim_end()
        )
    } else {
        extract
    };
    let _ = std::fs::write(&path, &truncated);
    Some(truncated)
}

/// Enriches the first `limit` POIs with Wikipedia extract as snippet (travelogue-style). Cached per title.
fn enrich_pois_with_extracts(
    pois: Vec<x_adox_core::flight_gen::PoiFile>,
    cache_dir: &Path,
    limit: usize,
) -> Vec<x_adox_core::flight_gen::PoiFile> {
    let fallback = Some(WIKIPEDIA_FALLBACK_PROXY_BASE);
    let mut out = pois;
    for (i, poi) in out.iter_mut().enumerate() {
        if i >= limit {
            break;
        }
        if let Some(extract) = get_extract_for_wikipedia_title(&poi.name, cache_dir, fallback) {
            poi.snippet = extract;
        }
    }
    out
}

/// Fetches POIs near (lat, lon) from Wikipedia geosearch API (radius 10 km, limit 20). Returns PoiFile list for merging into context.
/// If cache_dir is Some, reads/writes cache under cache_dir/pois_near (TTL 7 days) to avoid repeated requests.
pub fn fetch_pois_near_from_wikipedia(
    lat: f64,
    lon: f64,
    cache_dir: Option<&Path>,
) -> Option<Vec<x_adox_core::flight_gen::PoiFile>> {
    use std::time::{Duration, SystemTime};
    let cache_key = format!("{:.3}_{:.3}.json", lat, lon);
    if let Some(cache_dir) = cache_dir {
        let sub = cache_dir.join(POIS_NEAR_CACHE_SUBDIR);
        let path = sub.join(&cache_key);
        if let Ok(meta) = std::fs::metadata(&path) {
            if let Ok(mtime) = meta.modified() {
                let age = SystemTime::now()
                    .duration_since(mtime)
                    .unwrap_or(Duration::MAX);
                if age.as_secs() < POIS_NEAR_CACHE_TTL_SECS {
                    if let Ok(data) = std::fs::read_to_string(&path) {
                        if let Ok(pois) =
                            serde_json::from_str::<Vec<x_adox_core::flight_gen::PoiFile>>(&data)
                        {
                            if !pois.is_empty() {
                                return Some(pois);
                            }
                        }
                    }
                }
            }
        }
    }
    let gscoord = format!("{}|{}", lat, lon);
    let url = format!(
        "{}?action=query&list=geosearch&gscoord={}&gsradius=10000&gslimit=20&format=json",
        WIKIPEDIA_GEOSEARCH_URL.trim_end_matches('/'),
        urlencoding::encode(&gscoord)
    );
    let agent = poi_fetch_agent();
    let resp = agent
        .get(&url)
        .set("User-Agent", "X-Addon-Oxide/1.0 (flight context)")
        .call()
        .ok()?;
    let body = resp.into_string().ok()?;
    let v: serde_json::Value = serde_json::from_str(&body).ok()?;
    let geosearch = v.get("query")?.get("geosearch")?.as_array()?;
    let mut pois: Vec<x_adox_core::flight_gen::PoiFile> = Vec::with_capacity(geosearch.len());
    for item in geosearch {
        let Some(title) = item.get("title").and_then(|v| v.as_str()) else {
            continue;
        };
        // Filter out schools/academies/colleges
        let title_lower = title.to_lowercase();
        if title_lower.contains("school")
            || title_lower.contains("academy")
            || title_lower.contains("college")
            || title_lower.contains("university")
            || title_lower.contains("primary")
            || title_lower.contains("high school")
            || title_lower.contains("hospital")
            || title_lower.contains("f.c.")
            || title_lower.contains("football club")
            || title_lower.contains("roller coaster")
            || title_lower.contains("amusement ride")
            || title_lower == "rage"
        {
            continue;
        }
        let Some(lat_p) = item.get("lat").and_then(|v| v.as_f64()) else {
            continue;
        };
        let Some(lon_p) = item.get("lon").and_then(|v| v.as_f64()) else {
            continue;
        };
        pois.push(x_adox_core::flight_gen::PoiFile {
            name: title.to_string(),
            kind: "wikipedia".to_string(),
            snippet: title.to_string(),
            lat: lat_p,
            lon: lon_p,
            score: 10, // Base score for Wikipedia items
        });
    }
    if let Some(cache_dir) = cache_dir {
        if !pois.is_empty() {
            let sub = cache_dir.join(POIS_NEAR_CACHE_SUBDIR);
            let _ = std::fs::create_dir_all(&sub);
            let path = sub.join(&cache_key);
            let _ = std::fs::write(
                &path,
                serde_json::to_string_pretty(&pois).unwrap_or_default(),
            );
        }
    }
    Some(pois)
}

/// Parses WKT "Point(lon lat)" to (lat, lon). Returns None if format is wrong.
fn parse_wkt_point(wkt: &str) -> Option<(f64, f64)> {
    let s = wkt.trim();
    let inner = s.strip_prefix("Point(")?.strip_suffix(')')?;
    let parts: Vec<&str> = inner.split_whitespace().collect();
    if parts.len() >= 2 {
        let lon: f64 = parts[0].parse().ok()?;
        let lat: f64 = parts[1].parse().ok()?;
        Some((lat, lon))
    } else {
        None
    }
}

/// Extracts Wikipedia page title from en.wikipedia.org/wiki/... URL. Used for Wikidata sitelink.
fn wikipedia_title_from_url(url: &str) -> Option<String> {
    const PREFIX: &str = "https://en.wikipedia.org/wiki/";
    let s = url.trim();
    let title_encoded = s.strip_prefix(PREFIX)?;
    let title = urlencoding::decode(title_encoded).ok()?.into_owned();
    if title.is_empty() {
        return None;
    }
    Some(title)
}

/// Fetches POIs near (lat, lon) from Wikidata Query Service (SPARQL): stadiums, piers, museums, tourist attractions, landmarks within 20 km.
/// Only returns items that have an English Wikipedia sitelink so we can reuse extract enrichment. Cached like Wikipedia geosearch.
pub fn fetch_pois_near_from_wikidata(
    lat: f64,
    lon: f64,
    cache_dir: Option<&Path>,
) -> Option<Vec<x_adox_core::flight_gen::PoiFile>> {
    use std::time::{Duration, SystemTime};
    let cache_key = format!("{:.3}_{:.3}.json", lat, lon);
    if let Some(cache_dir) = cache_dir {
        let sub = cache_dir.join(POIS_NEAR_WIKIDATA_CACHE_SUBDIR);
        let path = sub.join(&cache_key);
        if let Ok(meta) = std::fs::metadata(&path) {
            if let Ok(mtime) = meta.modified() {
                let age = SystemTime::now()
                    .duration_since(mtime)
                    .unwrap_or(Duration::MAX);
                if age.as_secs() < POIS_NEAR_CACHE_TTL_SECS {
                    if let Ok(data) = std::fs::read_to_string(&path) {
                        if let Ok(pois) =
                            serde_json::from_str::<Vec<x_adox_core::flight_gen::PoiFile>>(&data)
                        {
                            if !pois.is_empty() {
                                return Some(pois);
                            }
                        }
                    }
                }
            }
        }
    }
    // WKT: Point(longitude latitude)
    let point = format!("Point({} {})", lon, lat);
    // Primary Query: Direct coordinates
    // Types: stadium, museum, tourist attraction, landmark, pier, football club, amusement park
    let query_direct = format!(
        r#"
PREFIX geo: <http://www.opengis.net/ont/geosparql#>
PREFIX wikibase: <http://wikiba.se/ontology#>
PREFIX wd: <http://www.wikidata.org/entity/>
PREFIX wdt: <http://www.wikidata.org/prop/direct/>
PREFIX schema: <http://schema.org/>
SELECT ?place ?placeLabel ?location ?article ?sitelinks ?type WHERE {{
  SERVICE wikibase:around {{
    ?place wdt:P625 ?location .
    bd:serviceParam wikibase:center "{point}"^^geo:wktLiteral .
    bd:serviceParam wikibase:radius "20" .
  }}
  ?place wdt:P31/wdt:P279* ?type .
  VALUES ?type {{ wd:Q483110 wd:Q33506 wd:Q570116 wd:Q2319498 wd:Q863454 wd:Q476028 wd:Q194195 }}
  ?place wikibase:sitelinks ?sitelinks .
  FILTER(?sitelinks > 3)
  ?article schema:about ?place .
  ?article schema:inLanguage "en" .
  ?article schema:isPartOf <https://en.wikipedia.org/> .
  SERVICE wikibase:label {{ bd:serviceParam wikibase:language "en". }}
}} ORDER BY DESC(?sitelinks)
LIMIT 40
"#
    );

    // Secondary Query: Tenants (Football Clubs via Venue)
    // Fixes missing coordinates for clubs like Southend United (Q48951)
    let query_tenants = format!(
        r#"
PREFIX geo: <http://www.opengis.net/ont/geosparql#>
PREFIX wikibase: <http://wikiba.se/ontology#>
PREFIX wd: <http://www.wikidata.org/entity/>
PREFIX wdt: <http://www.wikidata.org/prop/direct/>
PREFIX schema: <http://schema.org/>
SELECT ?place ?placeLabel ?location ?article ?sitelinks ?type ?venue WHERE {{
  SERVICE wikibase:around {{
    ?venue wdt:P625 ?location .
    bd:serviceParam wikibase:center "{point}"^^geo:wktLiteral .
    bd:serviceParam wikibase:radius "20" .
  }}
  ?place wdt:P115 ?venue .
  ?place wdt:P31/wdt:P279* ?type .
  VALUES ?type {{ wd:Q476028 }}
  ?place wikibase:sitelinks ?sitelinks .
  FILTER(?sitelinks > 3)
  ?article schema:about ?place .
  ?article schema:inLanguage "en" .
  ?article schema:isPartOf <https://en.wikipedia.org/> .
  SERVICE wikibase:label {{ bd:serviceParam wikibase:language "en". }}
}} ORDER BY DESC(?sitelinks)
LIMIT 20
"#
    );

    let mut candidates = Vec::new();
    let agent = poi_fetch_agent();

    // Track venues occupied by major clubs to hide the venue itself (deduplication)
    let mut occupied_venues = std::collections::HashSet::new();

    for (_q_idx, query) in [query_direct, query_tenants].iter().enumerate() {
        let url = format!(
            "{}?query={}&format=json",
            WIKIDATA_SPARQL_URL.trim_end_matches('/'),
            urlencoding::encode(query)
        );
        if let Ok(resp) = agent
            .get(&url)
            .set("User-Agent", "X-Addon-Oxide/1.0 (flight context)")
            .set("Accept", "application/sparql-results+json")
            .call()
        {
            if let Ok(body) = resp.into_string() {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(bindings) = v
                        .get("results")
                        .and_then(|r| r.get("bindings"))
                        .and_then(|b| b.as_array())
                    {
                        for row in bindings {
                            // Extract URI to check against occupied venues
                            let place_uri = row
                                .get("place")
                                .and_then(|n| n.get("value"))
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();

                            // If this is a tenant query result, record the venue URI
                            if let Some(venue_uri) = row
                                .get("venue")
                                .and_then(|n| n.get("value"))
                                .and_then(|v| v.as_str())
                            {
                                occupied_venues.insert(venue_uri.to_string());
                            }

                            let article_uri = match row
                                .get("article")
                                .and_then(|n| n.get("value"))
                                .and_then(|v| v.as_str())
                            {
                                Some(u) => u.to_string(),
                                None => continue,
                            };
                            let title = match wikipedia_title_from_url(&article_uri) {
                                Some(t) => t,
                                None => continue,
                            };
                            let type_uri = row
                                .get("type")
                                .and_then(|n| n.get("value"))
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string();
                            let sitelinks = row
                                .get("sitelinks")
                                .and_then(|n| n.get("value"))
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse::<i32>().ok())
                                .unwrap_or(0);

                            let location_str = match row
                                .get("location")
                                .and_then(|n| n.get("value"))
                                .and_then(|v| v.as_str())
                            {
                                Some(s) => s,
                                None => continue,
                            };
                            let (lat, lon) = match parse_wkt_point(location_str) {
                                Some(c) => c,
                                None => continue,
                            };

                            candidates.push((
                                x_adox_core::flight_gen::PoiFile {
                                    name: title,
                                    kind: "wikidata".to_string(), // we refine this later?
                                    snippet: String::new(),
                                    lat,
                                    lon,
                                    score: calculate_score(sitelinks, &type_uri),
                                },
                                sitelinks,
                                type_uri,
                                place_uri,
                            ));
                        }
                    }
                }
            }
        }
    }

    // Weighted Scoring Logic
    // Apply multipliers to base sitelinks to boost prominent types (e.g. Piers)
    // Multipliers: Pier (x5), Park (x1.5), Stadium (x1.5), Club (x1.5)
    candidates.sort_by(|a, b| {
        let score_a = calculate_score(a.1, &a.2);
        let score_b = calculate_score(b.1, &b.2);
        score_b.cmp(&score_a)
    });

    let mut pois = Vec::new();
    let mut football_clubs_count = 0;
    let mut seen_titles = std::collections::HashSet::new();

    for (poi, _sitelinks, type_uri, place_uri) in candidates {
        let name_lower = poi.name.to_lowercase();

        // Strict Name Filter to kill "Rage"
        if name_lower == "rage"
            || name_lower.contains("roller coaster")
            || name_lower.contains("amusement ride")
        {
            continue;
        }

        if !seen_titles.insert(name_lower) {
            continue;
        }

        // Filter out occupied venues (e.g. hide Roots Hall if Southend United is present)
        if occupied_venues.contains(&place_uri) {
            continue; // Skip this POI, it's just the building for a club we already have
        }

        // Limit football clubs (Q476028) to 1
        if type_uri.ends_with("Q476028") {
            if football_clubs_count >= 1 {
                continue;
            }
            football_clubs_count += 1;
        }
        pois.push(poi);
    }

    if let Some(cache_dir) = cache_dir {
        if !pois.is_empty() {
            let sub = cache_dir.join(POIS_NEAR_WIKIDATA_CACHE_SUBDIR);
            let _ = std::fs::create_dir_all(&sub);
            let path = sub.join(&cache_key);
            let _ = std::fs::write(
                &path,
                serde_json::to_string_pretty(&pois).unwrap_or_default(),
            );
        }
    }
    Some(pois)
}

fn calculate_score(sitelinks: i32, type_uri: &str) -> i32 {
    let base = sitelinks as f32;
    let multiplier = if type_uri.ends_with("Q863454") {
        5.0 // Generic Pier - Huge boost
    } else if type_uri.ends_with("Q194195") {
        1.5 // Amusement Park (or Theme Park)
    } else if type_uri.ends_with("Q483110") {
        1.5 // Stadium
    } else if type_uri.ends_with("Q476028") {
        1.5 // Football Club
    } else if type_uri.ends_with("Q2319498") {
        1.5 // Landmark
    } else {
        1.0
    };
    (base * multiplier) as i32
}

/// Merges Wikipedia and Wikidata POI lists and deduplicates by Wikipedia title (case-insensitive).
/// Wikipedia results come first; Wikidata fills gaps (e.g. Southend Pier, Roots Hall).
fn merge_pois_dedupe_by_title(
    wiki: Vec<x_adox_core::flight_gen::PoiFile>,
    wikidata: Vec<x_adox_core::flight_gen::PoiFile>,
) -> Vec<x_adox_core::flight_gen::PoiFile> {
    use std::collections::HashMap;
    let mut map: HashMap<String, x_adox_core::flight_gen::PoiFile> = HashMap::new();

    for p in wiki {
        map.insert(p.name.to_lowercase(), p);
    }

    for p in wikidata {
        let key = p.name.to_lowercase();
        match map.entry(key) {
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(p);
            }
            std::collections::hash_map::Entry::Occupied(mut e) => {
                let existing = e.get_mut();
                if p.score > existing.score {
                    existing.score = p.score;
                }
            }
        }
    }

    let mut out: Vec<_> = map.into_values().collect();
    // Sort by score DESC
    out.sort_by(|a, b| b.score.cmp(&a.score));
    out
}

/// Tries to fetch Wikipedia summary and parse extract. Returns (body, true if from fallback).
fn fetch_wikipedia_summary_body(
    primary_url: &str,
    fallback_proxy_base: Option<&str>,
) -> Option<String> {
    let agent = ureq::Agent::new();
    let resp = agent
        .get(primary_url)
        .set("User-Agent", "X-Addon-Oxide/1.0 (flight context)")
        .call();
    if let Ok(r) = resp {
        if let Ok(body) = r.into_string() {
            return Some(body);
        }
    }
    if let Some(base) = fallback_proxy_base {
        let fallback_url = format!("{}{}", base, urlencoding::encode(primary_url));
        if let Ok(r) = agent.get(&fallback_url).call() {
            if let Ok(body) = r.into_string() {
                return Some(body);
            }
        }
    }
    None
}

/// Fetches one airport's snippet from Wikipedia summary API; on primary failure tries fallback proxy if set.
/// Saves to cache and returns the context file. Returns None if title unknown, all requests fail, or no extract.
fn fetch_airport_context_from_wikipedia(
    icao: &str,
    title: &str,
    cache_dir: &Path,
    fallback_proxy_base: Option<&str>,
) -> Option<x_adox_core::flight_gen::AirportContextFile> {
    use x_adox_core::flight_gen::save_airport_context_to_cache;
    let primary_url = format!(
        "{}/{}",
        WIKIPEDIA_SUMMARY_URL.trim_end_matches('/'),
        urlencoding::encode(title)
    );
    let body = fetch_wikipedia_summary_body(&primary_url, fallback_proxy_base)?;
    let v: serde_json::Value = serde_json::from_str(&body).ok()?;
    let extract = v.get("extract")?.as_str()?.to_string();
    if extract.is_empty() {
        return None;
    }
    let data = x_adox_core::flight_gen::AirportContextFile {
        snippet: extract,
        points_nearby: vec![],
    };
    let _ = save_airport_context_to_cache(cache_dir, icao, &data);
    Some(data)
}

/// Loads context from bundled + config + cache; if enhanced, fills missing snippets from Wikipedia and fetches nearby POIs via geosearch, then re-loads.
/// Call from a background task when enhanced is on to avoid blocking the UI.
pub fn load_or_fetch_flight_context_blocking(
    config_root: PathBuf,
    origin: Airport,
    destination: Airport,
    enhanced_from_wikipedia: bool,
) -> Result<FlightContext, String> {
    let bundled = flight_gen::get_bundled_flight_context();
    let config_path = config_root.join("flight_context.json");
    let cache_dir = config_root.join(FLIGHT_CONTEXT_CACHE_DIR);

    let dynamic_origin_pois = if enhanced_from_wikipedia {
        flight_gen::airport_coords_for_poi_fetch(&origin).map(|(lat, lon)| {
            let wiki = fetch_pois_near_from_wikipedia(lat, lon, Some(&cache_dir)).unwrap_or_default();
            let wd = fetch_pois_near_from_wikidata(lat, lon, Some(&cache_dir)).unwrap_or_default();
            if wiki.is_empty() && wd.is_empty() {
                log::warn!("[flight_context] Origin {}: Wikipedia and Wikidata POI fetch both empty (lat={}, lon={})", origin.id, lat, lon);
            }
            let merged = merge_pois_dedupe_by_title(wiki, wd);
            enrich_pois_with_extracts(merged, &cache_dir, 8)
        })
    } else {
        None
    };
    let dynamic_dest_pois = if enhanced_from_wikipedia {
        flight_gen::airport_coords_for_poi_fetch(&destination).map(|(lat, lon)| {
            let wiki = fetch_pois_near_from_wikipedia(lat, lon, Some(&cache_dir)).unwrap_or_default();
            let wd = fetch_pois_near_from_wikidata(lat, lon, Some(&cache_dir)).unwrap_or_default();
            if wiki.is_empty() && wd.is_empty() {
                log::warn!("[flight_context] Destination {}: Wikipedia and Wikidata POI fetch both empty (lat={}, lon={})", destination.id, lat, lon);
            }
            let merged = merge_pois_dedupe_by_title(wiki, wd);
            enrich_pois_with_extracts(merged, &cache_dir, 8)
        })
    } else {
        None
    };

    let ctx = flight_gen::load_flight_context_with_bundled(
        &bundled,
        &config_path,
        &cache_dir,
        &origin,
        &destination,
        dynamic_origin_pois.clone(),
        dynamic_dest_pois.clone(),
    )
    .ok_or_else(|| "Failed to build context (missing coordinates?)".to_string())?;

    if enhanced_from_wikipedia {
        let map = flight_gen::get_icao_to_wikipedia();
        for icao in [&origin.id, &destination.id] {
            let has_snippet = (icao.eq_ignore_ascii_case(&origin.id)
                && !ctx.origin.snippet.is_empty())
                || (icao.eq_ignore_ascii_case(&destination.id)
                    && !ctx.destination.snippet.is_empty());
            if !has_snippet {
                if let Some(title) = map.get(icao).or_else(|| map.get(&icao.to_uppercase())) {
                    let _ = fetch_airport_context_from_wikipedia(
                        icao,
                        title,
                        &cache_dir,
                        Some(WIKIPEDIA_FALLBACK_PROXY_BASE),
                    );
                }
            }
        }
        return flight_gen::load_flight_context_with_bundled(
            &bundled,
            &config_path,
            &cache_dir,
            &origin,
            &destination,
            dynamic_origin_pois,
            dynamic_dest_pois,
        )
        .ok_or_else(|| "Failed to build context after fetch.".to_string());
    }
    Ok(ctx)
}
