use crate::style;
use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length, Padding, Task};
use rust_i18n::t;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
    /// Base airport layer: loaded asynchronously from X-Plane's Global Airports/Resources when the
    /// FlightGenerator tab is first opened. None = not yet loaded; Some = ready.
    pub base_airports: Option<std::sync::Arc<Vec<Airport>>>,
    /// Loading status message shown while apt.dat is being parsed in the background.
    pub base_airports_loading: Option<String>,
    /// When true, main should run enhanced context fetch if the setting is on (so history appears without clicking "Fetch context").
    pub pending_auto_fetch: bool,
    /// Tree view: Origin section expanded in History & context.
    pub origin_context_expanded: bool,
    /// Tree view: Destination section expanded in History & context.
    pub dest_context_expanded: bool,
    /// Tree view: Top-level History & context section expanded.

    /// As-of-now weather (decoded METAR) for departure airport. Set when context/weather is fetched.
    pub origin_weather: Option<String>,
    /// As-of-now weather (decoded METAR) for destination airport.
    pub dest_weather: Option<String>,
    /// If set, shows a modal with this text (for "Show full context").
    pub full_context_modal_text: Option<String>,
    /// If set, shows a temporary notification like "Copied!" for the copy button.
    pub context_copy_feedback: Option<std::time::Instant>,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
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
    /// Toggles the floating context window (handled in main).
    ToggleFlightContextWindow,
    /// Fetch context from API (Phase 2b); handled in main with Task::run, then ContextFetched.
    FetchContext,
    /// Toggle Origin tree section expanded/collapsed in History & context.
    ToggleOriginContext,
    /// Toggle Destination tree section expanded/collapsed in History & context.
    ToggleDestinationContext,
    /// Weather fetched for current plan (origin, dest). Called from main after FlightWeatherFetched.
    WeatherFetched(Option<String>, Option<String>),

    /// Copy the full context text to clipboard.
    CopyContext,
    /// Open a modal to show the full context text.
    ShowFullContext(String),
    /// Close the full context modal.
    CloseFullContext,
    /// Result of clipboard copy (internal use).
    CopyContextDone,
    /// Result of background flight generation.
    FlightGenerated(Box<Result<FlightPlan, String>>),
}

impl Default for FlightGenState {
    fn default() -> Self {
        Self {
            input_value: String::new(),
            history: vec![ChatMessage {
                text: t!("flight.welcome").to_string(),
                is_user: false,
            }],
            current_plan: None,
            status_message: None,
            base_airports: None,
            base_airports_loading: None,
            pending_auto_fetch: false,
            origin_context_expanded: true,
            dest_context_expanded: true,

            origin_weather: None,
            dest_weather: None,
            full_context_modal_text: None,
            context_copy_feedback: None,
        }
    }
}

/// True when the plan has no history content to show (no snippets, no POIs).
use x_adox_core::scenery::SceneryPack;

impl FlightGenState {
    pub fn update(
        &mut self,
        message: Message,
        packs: &Arc<Vec<SceneryPack>>,
        aircraft_list: &Arc<Vec<DiscoveredAddon>>,
        _xplane_root: Option<&Path>,
        prefs: Option<&HeuristicsConfig>,
        nlp_rules: Option<&x_adox_bitnet::NLPRulesConfig>,
    ) -> Task<Message> {
        match message {
            Message::InputChanged(val) => {
                self.input_value = val;
                Task::none()
            }
            Message::Submit => {
                if self.input_value.trim().is_empty() {
                    return Task::none();
                }

                let prompt = self.input_value.clone();
                self.history.push(ChatMessage {
                    text: prompt.clone(),
                    is_user: true,
                });
                self.input_value.clear();
                self.status_message = Some("Generating flight plan...".to_string());

                let packs = packs.clone();
                let aircraft_list = aircraft_list.clone();
                let base = self.base_airports.clone();
                let prefs = prefs.cloned();
                let nlp = nlp_rules.cloned();

                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let base_slice = base.as_deref().map(|v| v.as_slice());
                            flight_gen::generate_flight(
                                &packs,
                                &aircraft_list,
                                &prompt,
                                base_slice,
                                prefs.as_ref(),
                                nlp.as_ref(),
                            )
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |res| Message::FlightGenerated(Box::new(res)),
                )
            }
            Message::Regenerate => {
                let prompt = self
                    .history
                    .iter()
                    .rev()
                    .find(|m| m.is_user)
                    .map(|m| m.text.clone());
                if let Some(prompt) = prompt {
                    self.status_message = Some("Regenerating flight plan...".to_string());
                    let packs = packs.clone();
                    let aircraft_list = aircraft_list.clone();
                    let base = self.base_airports.clone();
                    let prefs = prefs.cloned();
                    let nlp = nlp_rules.cloned();

                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                let base_slice = base.as_deref().map(|v| v.as_slice());
                                flight_gen::generate_flight(
                                    &packs,
                                    &aircraft_list,
                                    &prompt,
                                    base_slice,
                                    prefs.as_ref(),
                                    nlp.as_ref(),
                                )
                            })
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                        },
                        |res| Message::FlightGenerated(Box::new(res)),
                    )
                } else {
                    Task::none()
                }
            }
            Message::FlightGenerated(box_res) => {
                let res = *box_res;
                // self.is_generating = false; // This line was commented out in the original context, keeping it that way.
                match res {
                    Ok(mut plan) => {
                        if let Some(ctx) = load_flight_context_for_plan(
                            get_config_root().as_path(),
                            &plan.origin,
                            &plan.destination,
                        ) {
                            plan.context = Some(ctx);
                        }
                        let time_label = plan.time.as_ref().map(|t| {
                            use x_adox_bitnet::flight_prompt::TimeKeyword;
                            match t {
                                TimeKeyword::Dawn => " · Dawn",
                                TimeKeyword::Day => " · Daytime",
                                TimeKeyword::Dusk => " · Dusk",
                                TimeKeyword::Night => " · Night",
                            }
                        });
                        // Only label confirmed weather (verified via live METAR).
                        // Unconfirmed requests (METAR unavailable) are not shown as fact.
                        let weather_label = plan
                            .weather
                            .as_ref()
                            .filter(|_| plan.weather_confirmed)
                            .map(|w| {
                                use x_adox_bitnet::flight_prompt::WeatherKeyword;
                                match w {
                                    WeatherKeyword::Clear => " · Clear",
                                    WeatherKeyword::Cloudy => " · Cloudy",
                                    WeatherKeyword::Storm => " · Storm",
                                    WeatherKeyword::Rain => " · Rain",
                                    WeatherKeyword::Snow => " · Snow",
                                    WeatherKeyword::Fog => " · Fog",
                                    WeatherKeyword::Gusty => " · Gusty",
                                    WeatherKeyword::Calm => " · Calm",
                                }
                            });
                        let conditions_suffix = format!(
                            "{}{}",
                            time_label.unwrap_or(""),
                            weather_label.unwrap_or("")
                        );
                        let response = format!(
                            "Generated Flight{conditions_suffix}:\nOrigin: {} ({})\nDestination: {} ({})\nAircraft: {}\nDistance: {} nm\nDuration: {} mins",
                            plan.origin.id, plan.origin.name,
                            plan.destination.id, plan.destination.name,
                            plan.aircraft.name,
                            plan.distance_nm,
                            plan.duration_minutes
                        );
                        self.history.push(ChatMessage {
                            text: response,
                            is_user: false,
                        });
                        self.current_plan = Some(plan);
                        self.origin_context_expanded = true;
                        self.dest_context_expanded = true;
                        self.pending_auto_fetch = true;
                        self.status_message = Some("Flight generated successfully.".to_string());
                    }
                    Err(e) => {
                        let mut text = format!("Error: {}", e);
                        // We can't access `packs` easily here for the "Global Airports" hint check
                        // without making it more complex, but we can check if the error is about suitable airports.
                        if e.contains("No suitable departure")
                            || e.contains("No suitable destination")
                        {
                            text.push_str("\nTip: Ensure airports are installed and enabled.");
                        }

                        self.history.push(ChatMessage {
                            text,
                            is_user: false,
                        });
                        self.status_message = Some("Generation failed.".to_string());
                    }
                }
                Task::none()
            }
            Message::ExportFms11
            | Message::ExportFms12
            | Message::ExportLnm
            | Message::ExportSimbrief => {
                // Export dialog logic is handled by the main app wrapper in main.rs
                Task::none()
            }
            // Handled in main (Task::run + FlightContextFetched).
            Message::FetchContext => Task::none(),
            Message::ToggleOriginContext => {
                self.origin_context_expanded = !self.origin_context_expanded;
                Task::none()
            }
            Message::ToggleDestinationContext => {
                self.dest_context_expanded = !self.dest_context_expanded;
                Task::none()
            }
            Message::WeatherFetched(origin, dest) => {
                self.origin_weather = origin;
                self.dest_weather = dest;
                Task::none()
            }

            // Handled in main (mutate heuristics_model); no-op here.
            Message::RememberThisFlight
            | Message::PreferThisOrigin
            | Message::PreferThisDestination => Task::none(),
            Message::CopyContext => {
                if let Some(plan) = &self.current_plan {
                    if let Some(ctx) = &plan.context {
                        let mut full_text = String::new();
                        full_text.push_str(&format!(
                            "Origin: {} ({})\n",
                            plan.origin.name, plan.origin.id
                        ));
                        full_text.push_str(&format!(
                            "Weather: {}\n",
                            self.origin_weather.as_deref().unwrap_or("N/A")
                        ));
                        full_text.push_str(&ctx.origin.snippet);
                        full_text.push_str("\n\nLandmarks:\n");
                        for poi in &ctx.origin.points_nearby {
                            full_text.push_str(&format!("- {} ({})\n", poi.name, poi.snippet));
                        }

                        full_text.push_str("\n----------------\n\n");

                        full_text.push_str(&format!(
                            "Destination: {} ({})\n",
                            plan.destination.name, plan.destination.id
                        ));
                        full_text.push_str(&format!(
                            "Weather: {}\n",
                            self.dest_weather.as_deref().unwrap_or("N/A")
                        ));
                        full_text.push_str(&ctx.destination.snippet);
                        full_text.push_str("\n\nLandmarks:\n");
                        for poi in &ctx.destination.points_nearby {
                            full_text.push_str(&format!("- {} ({})\n", poi.name, poi.snippet));
                        }

                        self.context_copy_feedback = Some(std::time::Instant::now());
                        return iced::clipboard::write(full_text)
                            .map(|_: ()| Message::CopyContextDone);
                    }
                }
                Task::none()
            }
            Message::CopyContextDone => Task::none(),
            Message::ToggleFlightContextWindow => Task::none(),
            Message::ShowFullContext(text) => {
                self.full_context_modal_text = Some(text);
                Task::none()
            }
            Message::CloseFullContext => {
                self.full_context_modal_text = None;
                Task::none()
            }
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
                    "Context loaded. No landmarks found — try Fetch Context again (check network)."
                        .to_string()
                } else if no_origin_landmarks || no_dest_landmarks {
                    "Context loaded. Some landmarks missing — try Fetch Context again.".to_string()
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
        // If modal open, show it on top of everything
        if let Some(full_text) = &self.full_context_modal_text {
            let modal_content = container(
                column![
                    row![
                        text(t!("flight.context.title"))
                            .size(20)
                            .color(style::palette::TEXT_PRIMARY)
                            .width(Length::Fill),
                        button(text("X").size(16))
                            .on_press(Message::CloseFullContext)
                            .style(style::button_ghost)
                            .padding(5)
                    ]
                    .align_y(iced::Alignment::Center),
                    scrollable(text(full_text).size(14).line_height(1.6).style(move |_| {
                        iced::widget::text::Style {
                            color: Some(style::palette::TEXT_PRIMARY),
                        }
                    }))
                    .height(Length::Fill),
                    button(text(t!("btn.close")).size(14))
                        .on_press(Message::CloseFullContext)
                        .style(style::button_secondary)
                        .padding([8, 16])
                        .width(Length::Fill)
                ]
                .spacing(15),
            )
            .style(style::container_modal)
            .padding(20)
            .width(Length::FillPortion(1))
            .height(Length::FillPortion(1)); // Make it take good space

            // Center modal in a larger container
            return container(modal_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgba(
                        0.0, 0.0, 0.0, 0.7,
                    ))),
                    ..Default::default()
                })
                .into();
        }

        let chat_history = scrollable(
            column(self.history.iter().map(|msg| {
                container(column![
                    text(if msg.is_user {
                        t!("flight.sender_user").to_string()
                    } else {
                        t!("flight.sender_system").to_string()
                    })
                    .size(12)
                    .style(move |_| if msg.is_user {
                        iced::widget::text::Style {
                            color: Some(iced::Color::from_rgb(0.6, 0.8, 1.0)), // Lighter blue for User
                        }
                    } else {
                        iced::widget::text::Style {
                            color: Some(iced::Color::from_rgb(0.6, 1.0, 0.8)), // Mint for System
                        }
                    }),
                    text(&msg.text).size(15).line_height(1.5)
                ])
                .padding(10)
                .style(move |_| {
                    if msg.is_user {
                        container::Style {
                            background: Some(iced::Background::Color(iced::Color::from_rgb(
                                0.18, 0.18, 0.22,
                            ))),
                            border: iced::Border {
                                radius: 8.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    } else {
                        container::Style {
                            background: Some(iced::Background::Color(iced::Color::from_rgb(
                                0.15, 0.15, 0.15,
                            ))),
                            border: iced::Border {
                                radius: 8.0.into(),
                                ..Default::default()
                            },
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

        let flight_placeholder = t!("flight.placeholder");
        let input_area = row![
            text_input(&flight_placeholder, &self.input_value)
                .on_input(Message::InputChanged)
                .on_submit(Message::Submit)
                .padding(10)
                .style(style::text_input_primary),
            button(text(t!("flight.send")))
                .on_press(Message::Submit)
                .padding(10)
                .style(style::button_primary)
        ]
        .spacing(10);

        // Row 1: Regenerate + Export format buttons
        let mut export_controls = row![].spacing(10);

        // Always show Regenerate if we have at least one user prompt to regenerate from
        if self.history.iter().any(|m| m.is_user) {
            export_controls = export_controls.push(
                button(text(t!("flight.regenerate")))
                    .on_press(Message::Regenerate)
                    .style(style::button_success_glow),
            );
        }

        // Row 2: Learning + Context buttons (only shown when a plan exists)
        let mut learning_controls = row![].spacing(10);

        if let Some(plan) = &self.current_plan {
            export_controls = export_controls.push(
                button("FMS 11")
                    .on_press(Message::ExportFms11)
                    .style(style::button_magenta_glow),
            );
            export_controls = export_controls.push(
                button("FMS 12")
                    .on_press(Message::ExportFms12)
                    .style(style::button_cyan_glow),
            );
            export_controls = export_controls.push(
                button("LNM")
                    .on_press(Message::ExportLnm)
                    .style(style::button_orange_glow),
            );
            export_controls = export_controls.push(
                button("SimBrief")
                    .on_press(Message::ExportSimbrief)
                    .style(style::button_purple_glow),
            );

            if plan.origin_region_id.is_some() && plan.dest_region_id.is_some() {
                learning_controls = learning_controls.push(
                    button(text(t!("flight.remember")))
                        .on_press(Message::RememberThisFlight)
                        .style(style::button_ghost_amber),
                );
            }
            if plan.origin_region_id.is_some() {
                learning_controls = learning_controls.push(
                    button(text(t!("flight.prefer_origin")))
                        .on_press(Message::PreferThisOrigin)
                        .style(style::button_ghost_teal),
                );
            }
            if plan.dest_region_id.is_some() {
                learning_controls = learning_controls.push(
                    button(text(t!("flight.prefer_dest")))
                        .on_press(Message::PreferThisDestination)
                        .style(style::button_ghost_indigo),
                );
            }
            learning_controls = learning_controls.push(
                button(text(t!("flight.history_context")))
                    .on_press(Message::ToggleFlightContextWindow)
                    .style(style::button_primary_glow),
            );
        }

        let controls = column![export_controls, learning_controls].spacing(6);

        let mut col = column![
            container(chat_history)
                .height(Length::Fill)
                .style(style::container_main_content)
                .padding(5),
            iced::widget::horizontal_rule(1.0),
            controls,
        ]
        .spacing(15)
        .padding(20);

        // Show global airports loading indicator below the controls
        if let Some(loading_msg) = &self.base_airports_loading {
            col = col.push(
                text(loading_msg)
                    .size(12)
                    .style(|_| iced::widget::text::Style {
                        color: Some(iced::Color::from_rgb(0.5, 0.8, 0.5)),
                    }),
            );
        } else if self.base_airports.is_some() {
            // Subtle indicator that global airports are ready
            let count = self.base_airports.as_ref().map(|v| v.len()).unwrap_or(0);
            col = col.push(
                text(format!("Global airport database: {} airports", count))
                    .size(11)
                    .style(|_| iced::widget::text::Style {
                        color: Some(iced::Color::from_rgb(0.35, 0.55, 0.35)),
                    }),
            );
        }

        col = col.push(input_area);
        col.into()
    }

    /// Renders just the origin and destination details for the context window or inline panel.
    pub fn view_context_content<'a>(&'a self, plan: &'a FlightPlan) -> Element<'a, Message> {
        // 2. Core content (Origin + Destination)
        let (origin_inner, dest_inner) = match &plan.context {
            Some(ctx) => (self.origin_tree_content(ctx), self.dest_tree_content(ctx)),
            None => (
                column![
                    text(t!("flight.context.no_context"))
                        .size(12)
                        .color(style::palette::TEXT_SECONDARY),
                    text(t!("flight.context.weather_now"))
                        .size(11)
                        .color(iced::Color::from_rgb(0.6, 0.6, 0.65)),
                    text(self.origin_weather.as_deref().unwrap_or("—")).size(12),
                ]
                .spacing(6)
                .into(),
                column![
                    text(t!("flight.context.no_context"))
                        .size(12)
                        .color(style::palette::TEXT_SECONDARY),
                    text(t!("flight.context.weather_now"))
                        .size(11)
                        .color(iced::Color::from_rgb(0.6, 0.6, 0.65)),
                    text(self.dest_weather.as_deref().unwrap_or("—")).size(12),
                ]
                .spacing(6)
                .into(),
            ),
        };

        let origin_header = row![
            button(
                text(if self.origin_context_expanded {
                    "v"
                } else {
                    ">"
                })
                .size(12)
            )
            .on_press(Message::ToggleOriginContext)
            .padding(2)
            .style(style::button_ghost),
            button(
                text(format!("Origin: {} ({})", plan.origin.name, plan.origin.id))
                    .size(13)
                    .style(move |_| iced::widget::text::Style {
                        color: Some(style::palette::ACCENT_BLUE)
                    })
            )
            .on_press(Message::ToggleOriginContext)
            .padding(2)
            .style(style::button_ghost),
        ]
        .spacing(4);

        let dest_header = row![
            button(text(if self.dest_context_expanded { "v" } else { ">" }).size(12))
                .on_press(Message::ToggleDestinationContext)
                .padding(2)
                .style(style::button_ghost),
            button(
                text(format!(
                    "Destination: {} ({})",
                    plan.destination.name, plan.destination.id
                ))
                .size(13)
                .style(move |_| iced::widget::text::Style {
                    color: Some(style::palette::ACCENT_BLUE)
                })
            )
            .on_press(Message::ToggleDestinationContext)
            .padding(2)
            .style(style::button_ghost),
        ]
        .spacing(4);

        let origin_section = column![
            origin_header,
            if self.origin_context_expanded {
                column![origin_inner].padding(Padding {
                    top: 8.0,
                    bottom: 4.0,
                    left: 16.0,
                    right: 0.0,
                })
            } else {
                column![]
            },
        ]
        .spacing(4);

        let dest_section = column![
            dest_header,
            if self.dest_context_expanded {
                column![dest_inner].padding(Padding {
                    top: 8.0,
                    bottom: 4.0,
                    left: 16.0,
                    right: 0.0,
                })
            } else {
                column![]
            },
        ]
        .spacing(4);

        let enroute_note = text("En route: Check NOTAMs and winds aloft for your route.")
            .size(11)
            .color(style::palette::TEXT_SECONDARY);

        column![origin_section, dest_section, enroute_note]
            .spacing(16)
            .into()
    }

    fn origin_tree_content<'a>(&'a self, ctx: &'a FlightContext) -> Element<'a, Message> {
        let history_label =
            text(t!("flight.context.history"))
                .size(11)
                .style(move |_| iced::widget::text::Style {
                    color: Some(iced::Color::from_rgb(0.6, 0.6, 0.65)),
                });

        let mut history_elements = vec![];
        if ctx.origin.snippet.is_empty() {
            history_elements.push(
                text(t!("flight.context.no_wikipedia"))
                    .size(12)
                    .color(style::palette::TEXT_SECONDARY)
                    .width(Length::Fill)
                    .into(),
            );
        } else {
            // Typography upgrade: 13.5px size, 1.7 line height (approx via spacing)
            // Truncation check
            let snippet = &ctx.origin.snippet;

            // Show snippet
            history_elements.push(
                text(snippet)
                    .size(13)
                    .line_height(1.6)
                    .width(Length::Fill)
                    .into(),
            );

            if snippet.len() > 1500 {
                history_elements.push(
                    button(
                        text(t!("flight.context.show_full"))
                            .size(12)
                            .color(style::palette::ACCENT_BLUE),
                    )
                    .on_press(Message::ShowFullContext(snippet.clone()))
                    .style(style::button_ghost)
                    .padding(0)
                    .into(),
                );
            }
        };

        let landmarks_label =
            text(t!("flight.context.landmarks_title"))
                .size(11)
                .style(move |_| iced::widget::text::Style {
                    color: Some(iced::Color::from_rgb(0.6, 0.6, 0.65)),
                });
        let landmarks_attribution = text(t!("flight.context.landmarks_attr"))
            .size(10)
            .color(style::palette::TEXT_SECONDARY);

        let landmarks_list: Vec<Element<_>> = if ctx.origin.points_nearby.is_empty() {
            vec![text(t!("flight.context.no_landmarks"))
                .size(12)
                .color(style::palette::TEXT_SECONDARY)
                .into()]
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
                        .size(13)
                        .line_height(1.5)
                        .color(style::palette::TEXT_PRIMARY)
                        .width(Length::Fill)
                        .into()
                })
                .collect()
        };

        let weather_label = text(t!("flight.context.weather_now"))
            .size(11)
            .style(move |_| iced::widget::text::Style {
                color: Some(iced::Color::from_rgb(0.6, 0.6, 0.65)),
            });
        let weather_body = text(
            self.origin_weather
                .as_deref()
                .map(|s| s.to_owned())
                .unwrap_or_else(|| t!("flight.context.fetch_metar").into_owned()),
        )
        .size(13)
        .width(Length::Fill);

        column![
            column![history_label, column(history_elements).spacing(5)].spacing(4),
            iced::widget::horizontal_rule(1.0),
            column![
                landmarks_label,
                landmarks_attribution,
                column(landmarks_list).spacing(6)
            ]
            .spacing(4),
            iced::widget::horizontal_rule(1.0),
            column![weather_label, weather_body].spacing(4),
        ]
        .spacing(12)
        .into()
    }

    fn dest_tree_content<'a>(&'a self, ctx: &'a FlightContext) -> Element<'a, Message> {
        let history_label =
            text(t!("flight.context.history"))
                .size(11)
                .style(move |_| iced::widget::text::Style {
                    color: Some(iced::Color::from_rgb(0.6, 0.6, 0.65)),
                });

        let mut history_elements = vec![];
        if ctx.destination.snippet.is_empty() {
            history_elements.push(
                text(t!("flight.context.no_wikipedia"))
                    .size(12)
                    .color(style::palette::TEXT_SECONDARY)
                    .width(Length::Fill)
                    .into(),
            );
        } else {
            let snippet = &ctx.destination.snippet;

            history_elements.push(
                text(snippet)
                    .size(13)
                    .line_height(1.6)
                    .width(Length::Fill)
                    .into(),
            );

            if snippet.len() > 1500 {
                history_elements.push(
                    button(
                        text(t!("flight.context.show_full"))
                            .size(12)
                            .color(style::palette::ACCENT_BLUE),
                    )
                    .on_press(Message::ShowFullContext(snippet.clone()))
                    .style(style::button_ghost)
                    .padding(0)
                    .into(),
                );
            }
        };

        let landmarks_label =
            text(t!("flight.context.landmarks_title"))
                .size(11)
                .style(move |_| iced::widget::text::Style {
                    color: Some(iced::Color::from_rgb(0.6, 0.6, 0.65)),
                });
        let landmarks_attribution = text(t!("flight.context.landmarks_attr"))
            .size(10)
            .color(style::palette::TEXT_SECONDARY);
        let landmarks_list: Vec<Element<_>> = if ctx.destination.points_nearby.is_empty() {
            vec![text(t!("flight.context.no_landmarks"))
                .size(12)
                .color(style::palette::TEXT_SECONDARY)
                .into()]
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
                        .size(13)
                        .line_height(1.5)
                        .color(style::palette::TEXT_PRIMARY)
                        .width(Length::Fill)
                        .into()
                })
                .collect()
        };
        let weather_label = text(t!("flight.context.weather_now"))
            .size(11)
            .style(move |_| iced::widget::text::Style {
                color: Some(iced::Color::from_rgb(0.6, 0.6, 0.65)),
            });
        let weather_body = text(
            self.dest_weather
                .as_deref()
                .map(|s| s.to_owned())
                .unwrap_or_else(|| t!("flight.context.fetch_metar").into_owned()),
        )
        .size(13)
        .width(Length::Fill);

        column![
            column![history_label, column(history_elements).spacing(5)].spacing(4),
            iced::widget::horizontal_rule(1.0),
            column![
                landmarks_label,
                landmarks_attribution,
                column(landmarks_list).spacing(6)
            ]
            .spacing(4),
            iced::widget::horizontal_rule(1.0),
            column![weather_label, weather_body].spacing(4),
        ]
        .spacing(12)
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

/// Fetches raw METAR for origin and destination ICAO. Returns (origin_text, dest_text).
/// Primary source: local bulk METAR cache (global coverage, downloaded by WeatherEngine).
/// Fallback: AWC point API (covers US airports when the cache is missing or stale).
pub fn fetch_weather_for_plan(
    origin_icao: &str,
    dest_icao: &str,
) -> (Option<String>, Option<String>) {
    let origin_icao_up = origin_icao.trim().to_uppercase();
    let dest_icao_up = dest_icao.trim().to_uppercase();

    // --- Primary: local bulk METAR cache (has global coverage including EGLL, ZBNY, etc.) ---
    let engine = x_adox_core::weather::WeatherEngine::new();
    let ids_slice: Vec<&str> = vec![origin_icao_up.as_str(), dest_icao_up.as_str()];
    let mut metar_map = engine.get_raw_metars(&ids_slice);

    // For 3-char US airports, also try the K-prefixed version
    if !metar_map.contains_key(&origin_icao_up) && origin_icao_up.len() == 3 {
        let k_ids: Vec<&str> = vec![&origin_icao_up];
        let k_origin = format!("K{}", origin_icao_up);
        let k_ids2: Vec<&str> = vec![k_origin.as_str()];
        let r = engine.get_raw_metars(&k_ids2);
        if let Some(v) = r.get(&k_origin) {
            metar_map.insert(origin_icao_up.clone(), v.clone());
        }
        let _ = k_ids; // suppress unused warning
    }
    if !metar_map.contains_key(&dest_icao_up) && dest_icao_up.len() == 3 {
        let k_dest = format!("K{}", dest_icao_up);
        let k_ids: Vec<&str> = vec![k_dest.as_str()];
        let r = engine.get_raw_metars(&k_ids);
        if let Some(v) = r.get(&k_dest) {
            metar_map.insert(dest_icao_up.clone(), v.clone());
        }
    }

    // --- Fallback: AWC point API for any ICAOs still missing (US airport coverage) ---
    let need_api: Vec<String> = [&origin_icao_up, &dest_icao_up]
        .iter()
        .filter(|id| !metar_map.contains_key(id.as_str()))
        .map(|id| id.to_string())
        .collect();

    if !need_api.is_empty() {
        let mut api_ids = need_api.clone();
        for id in &need_api {
            if id.len() == 3 {
                api_ids.push(format!("K{}", id));
            }
        }
        let ids_str = api_ids.join(",");
        let url = format!(
            "{}?ids={}&format=raw",
            AWC_METAR_URL,
            urlencoding::encode(&ids_str)
        );
        let agent = ureq::Agent::new();
        if let Ok(resp) = agent
            .get(&url)
            .set("User-Agent", "X-Addon-Oxide/1.0 (flight context)")
            .call()
        {
            if let Ok(body) = resp.into_string() {
                for line in body.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some(icao) = line.split_whitespace().next() {
                        let key = icao.trim().to_uppercase();
                        // Match directly or via K-prefix strip
                        if need_api.contains(&key) {
                            metar_map.entry(key).or_insert_with(|| line.to_string());
                        } else if key.starts_with('K') && key.len() == 4 {
                            let bare = &key[1..];
                            if need_api.iter().any(|id| id == bare) {
                                metar_map
                                    .entry(bare.to_string())
                                    .or_insert_with(|| line.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    let origin = metar_map.get(&origin_icao_up).cloned().or_else(|| {
        Some(format!(
            "No METAR available for {} (no reporting station).",
            origin_icao_up
        ))
    });
    let dest = metar_map.get(&dest_icao_up).cloned().or_else(|| {
        Some(format!(
            "No METAR available for {} (no reporting station).",
            dest_icao_up
        ))
    });

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

    for query in [query_direct, query_tenants].iter() {
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
    } else if type_uri.ends_with("Q194195")
        || type_uri.ends_with("Q483110")
        || type_uri.ends_with("Q476028")
        || type_uri.ends_with("Q2319498")
    {
        1.5 // Amusement Park, Stadium, Football Club, Landmark
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
