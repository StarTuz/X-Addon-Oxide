use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};
use x_adox_core::discovery::DiscoveredAddon;
use x_adox_core::flight_gen::{self, FlightPlan};

#[derive(Debug, Clone)]
pub struct FlightGenState {
    pub input_value: String,
    pub history: Vec<ChatMessage>,
    pub current_plan: Option<FlightPlan>,
    pub status_message: Option<String>,
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
    ExportFms11,
    ExportFms12,
    ExportLnm,
    ExportSimbrief,
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
    ) {
        match message {
            Message::InputChanged(val) => {
                self.input_value = val;
            }
            Message::Submit => {
                if self.input_value.trim().is_empty() {
                    return;
                }
                let prompt = self.input_value.clone();
                self.history.push(ChatMessage {
                    sender: "User".to_string(),
                    text: prompt.clone(),
                    is_user: true,
                });
                self.input_value.clear();

                match flight_gen::generate_flight(packs, aircraft_list, &prompt) {
                    Ok(plan) => {
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
                        self.status_message = Some("Flight generated successfully.".to_string());
                    }
                    Err(e) => {
                        self.history.push(ChatMessage {
                            sender: "System".to_string(),
                            text: format!("Error: {}", e),
                            is_user: false,
                        });
                        self.status_message = Some(format!("Error: {}", e));
                    }
                }
            }
            Message::ExportFms11 => {
                if let Some(plan) = &self.current_plan {
                    let text = flight_gen::export_fms_11(plan);
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
        }
    }

    pub fn view(&self) -> Element<Message> {
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

        let controls = if self.current_plan.is_some() {
            row![
                button("FMS 11").on_press(Message::ExportFms11),
                button("FMS 12").on_press(Message::ExportFms12),
                button("LNM").on_press(Message::ExportLnm),
                button("SimBrief").on_press(Message::ExportSimbrief)
            ]
            .spacing(10)
        } else {
            row![]
        };

        column![chat_history, controls, input_area]
            .spacing(20)
            .padding(20)
            .into()
    }
}
