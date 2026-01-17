use iced::advanced::{self, layout, renderer, widget, Layout, Widget};
use iced::widget::{
    button, checkbox, column, container, image, responsive, row, scrollable, svg, text, Column,
};
use iced::{mouse, Color, Element, Event, Length, Radians, Rectangle, Task, Theme};
use std::path::PathBuf;
use x_adox_core::discovery::{AddonType, DiscoveredAddon, DiscoveryManager};
use x_adox_core::management::ModManager;
use x_adox_core::scenery::{SceneryManager, SceneryPack, SceneryPackType};
use x_adox_core::XPlaneManager;

mod style;
// use style::{Button as ButtonStyle, Container as ContainerStyle}; // Removed

fn main() -> iced::Result {
    iced::application("X-Addon-Oxide", App::update, App::view)
        .theme(|_| Theme::Dark)
        .run_with(App::new)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Scenery,
    Aircraft,
    Plugins,
    CSLs,
}

#[derive(Debug, Clone)]
struct AircraftNode {
    name: String,
    path: PathBuf,
    is_folder: bool,
    is_expanded: bool,
    children: Vec<AircraftNode>,
    acf_file: Option<String>, // .acf filename if aircraft
}

#[derive(Debug, Clone)]
enum Message {
    // Tab navigation
    SwitchTab(Tab),

    // Scenery
    SceneryLoaded(Result<Vec<SceneryPack>, String>),
    TogglePack(String),
    PackToggled(Result<(), String>),

    // Aircraft & Plugins
    AircraftLoaded(Result<Vec<DiscoveredAddon>, String>),
    PluginsLoaded(Result<Vec<DiscoveredAddon>, String>),
    TogglePlugin(PathBuf, bool),
    PluginToggled(Result<(), String>),
    CSLsLoaded(Result<Vec<DiscoveredAddon>, String>),
    ToggleCSL(PathBuf, bool),

    // Common
    Refresh,
    SelectFolder,
    FolderSelected(Option<PathBuf>),

    // Aircraft tree
    ToggleAircraftFolder(PathBuf),
    AircraftTreeLoaded(Result<AircraftNode, String>),

    // Install/Delete
    SelectScenery(String),
    HoverScenery(Option<String>),
    SelectAircraft(PathBuf),
    SelectPlugin(PathBuf),
    SelectCSL(PathBuf),
    InstallScenery,
    InstallAircraft,
    InstallPlugin,
    InstallCSL,
    InstallPicked(Tab, Option<PathBuf>),
    InstallAircraftDestPicked(PathBuf, Option<PathBuf>),
    InstallComplete(Result<String, String>),
    DeleteAddon(Tab),
    ConfirmDelete(Tab, bool),

    // Expansion & Scripts
    MapZoom {
        new_center: (f64, f64),
        new_zoom: f64,
    },
}

struct App {
    active_tab: Tab,
    packs: Vec<SceneryPack>,
    aircraft: Vec<DiscoveredAddon>,
    aircraft_tree: Option<AircraftNode>,
    plugins: Vec<DiscoveredAddon>,
    csls: Vec<DiscoveredAddon>,
    status: String,
    xplane_root: Option<PathBuf>,
    selected_scenery: Option<String>,
    selected_aircraft: Option<PathBuf>,
    selected_aircraft_icon: Option<image::Handle>,
    selected_plugin: Option<PathBuf>,
    selected_csl: Option<PathBuf>,
    show_delete_confirm: bool,
    show_csl_tab: bool,
    // Assets
    tile_manager: TileManager,
    icon_aircraft: svg::Handle,
    icon_scenery: svg::Handle,
    icon_plugins: svg::Handle,
    icon_csls: svg::Handle,
    refresh_icon: svg::Handle,
    // Map state
    hovered_scenery: Option<String>,
    map_zoom: f64,
    map_center: (f64, f64), // (lat, lon)
    map_initialized: bool,
    scenery_scroll_id: scrollable::Id,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let root = XPlaneManager::try_find_root();

        let app = Self {
            active_tab: Tab::Scenery,
            packs: Vec::new(),
            aircraft: Vec::new(),
            aircraft_tree: None,
            plugins: Vec::new(),
            csls: Vec::new(),
            status: "Loading...".to_string(),
            xplane_root: root.clone(),
            selected_scenery: None,
            selected_aircraft: None,
            selected_aircraft_icon: None,
            selected_plugin: None,
            selected_csl: None,
            show_delete_confirm: false,
            show_csl_tab: false,
            tile_manager: TileManager::new(),
            icon_aircraft: svg::Handle::from_memory(
                include_bytes!("../assets/icons/aircraft.svg").to_vec(),
            ),
            icon_scenery: svg::Handle::from_memory(
                include_bytes!("../assets/icons/scenery.svg").to_vec(),
            ),
            icon_plugins: svg::Handle::from_memory(
                include_bytes!("../assets/icons/plugins.svg").to_vec(),
            ),
            icon_csls: svg::Handle::from_memory(
                include_bytes!("../assets/icons/csls.svg").to_vec(),
            ),
            refresh_icon: svg::Handle::from_memory(
                include_bytes!("../assets/icons/refresh.svg").to_vec(),
            ),
            hovered_scenery: None,
            map_zoom: 0.0,
            map_center: (0.0, 0.0),
            map_initialized: false,
            scenery_scroll_id: scrollable::Id::unique(),
        };

        let tasks = if let Some(r) = root {
            let r1 = r.clone();
            let r2 = r.clone();
            let r3 = r.clone();
            let r4 = r.clone();
            Task::batch(vec![
                Task::perform(async move { load_packs(Some(r1)) }, Message::SceneryLoaded),
                Task::perform(
                    async move { load_aircraft_tree(Some(r2)) },
                    Message::AircraftTreeLoaded,
                ),
                Task::perform(
                    async move { load_plugins(Some(r3)) },
                    Message::PluginsLoaded,
                ),
                Task::perform(async move { load_csls(Some(r4)) }, Message::CSLsLoaded),
            ])
        } else {
            Task::none()
        };

        (app, tasks)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MapZoom {
                new_center,
                new_zoom,
            } => {
                self.map_center = new_center;
                self.map_zoom = new_zoom;
                self.map_initialized = true; // Mark as initialized on first manual interaction
                return Task::none();
            }
            Message::SwitchTab(tab) => {
                self.active_tab = tab;
                // Update status to reflect current tab
                self.status = match tab {
                    Tab::Scenery => format!("{} scenery packs", self.packs.len()),
                    Tab::Aircraft => format!("{} aircraft", self.aircraft.len()),
                    Tab::Plugins => format!("{} plugins", self.plugins.len()),
                    Tab::CSLs => format!("{} CSL packages", self.csls.len()),
                };
                Task::none()
            }
            Message::SceneryLoaded(result) => {
                match result {
                    Ok(packs) => {
                        self.packs = packs;
                        self.status = format!("{} scenery packs", self.packs.len());
                    }
                    Err(e) => self.status = format!("Scenery error: {}", e),
                }
                Task::none()
            }
            Message::AircraftLoaded(result) => {
                match result {
                    Ok(aircraft) => {
                        self.aircraft = aircraft;
                        if self.active_tab == Tab::Aircraft {
                            self.status = format!("{} aircraft", self.aircraft.len());
                        }
                    }
                    Err(e) => {
                        if self.active_tab == Tab::Aircraft {
                            self.status = format!("Aircraft error: {}", e);
                        }
                    }
                }
                Task::none()
            }
            Message::PluginsLoaded(result) => {
                match result {
                    Ok(plugins) => {
                        self.plugins = plugins;
                        if self.active_tab == Tab::Plugins {
                            self.status = format!("{} plugins", self.plugins.len());
                        }
                    }
                    Err(e) => {
                        if self.active_tab == Tab::Plugins {
                            self.status = format!("Plugins error: {}", e);
                        }
                    }
                }
                Task::none()
            }
            Message::CSLsLoaded(result) => {
                match result {
                    Ok(csls) => {
                        self.csls = csls;
                        self.show_csl_tab = !self.csls.is_empty();
                        if self.active_tab == Tab::CSLs {
                            self.status = format!("{} CSL packages", self.csls.len());
                        }
                    }
                    Err(e) => {
                        if self.active_tab == Tab::CSLs {
                            self.status = format!("CSL error: {}", e);
                        }
                    }
                }
                Task::none()
            }
            Message::AircraftTreeLoaded(result) => {
                match result {
                    Ok(tree) => {
                        self.aircraft_tree = Some(tree);
                        if self.active_tab == Tab::Aircraft {
                            self.status = "Aircraft tree loaded".to_string();
                        }
                    }
                    Err(e) => {
                        if self.active_tab == Tab::Aircraft {
                            self.status = format!("Aircraft tree error: {}", e);
                        }
                    }
                }
                Task::none()
            }
            Message::ToggleCSL(path, enable) => {
                let root = self.xplane_root.clone();
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                self.status = format!(
                    "{} CSL {}...",
                    if enable { "Enabling" } else { "Disabling" },
                    name
                );
                Task::perform(
                    async move { toggle_csl(root, path, enable) },
                    |result| match result {
                        Ok(_) => Message::Refresh,
                        Err(e) => Message::CSLsLoaded(Err(e)),
                    },
                )
            }
            Message::TogglePlugin(path, enable) => {
                let root = self.xplane_root.clone();
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                self.status = format!(
                    "{} Plugin {}...",
                    if enable { "Enabling" } else { "Disabling" },
                    name
                );
                Task::perform(
                    async move { toggle_plugin(root, path, enable) },
                    Message::PluginToggled,
                )
            }
            Message::PluginToggled(result) => match result {
                Ok(_) => {
                    self.status = "Plugin toggled!".to_string();
                    let root = self.xplane_root.clone();
                    Task::perform(async move { load_plugins(root) }, Message::PluginsLoaded)
                }
                Err(e) => {
                    self.status = format!("Error toggling plugin: {}", e);
                    Task::none()
                }
            },
            Message::SelectCSL(path) => {
                self.selected_csl = Some(path);
                Task::none()
            }
            Message::InstallCSL => Task::perform(pick_zip("Select CSL ZIP"), |p| {
                Message::InstallPicked(Tab::CSLs, p)
            }),
            Message::TogglePack(name) => {
                let root = self.xplane_root.clone();
                let enable = self
                    .packs
                    .iter()
                    .find(|p| p.name == name)
                    .map(|p| p.status == SceneryPackType::Disabled)
                    .unwrap_or(false);

                self.status = format!(
                    "{} {}...",
                    if enable { "Enabling" } else { "Disabling" },
                    name
                );

                Task::perform(
                    async move { toggle_pack(root, name, enable) },
                    Message::PackToggled,
                )
            }
            Message::PackToggled(result) => match result {
                Ok(()) => {
                    self.status = "Saved!".to_string();
                    let root = self.xplane_root.clone();
                    Task::perform(async move { load_packs(root) }, Message::SceneryLoaded)
                }
                Err(e) => {
                    self.status = format!("Error: {}", e);
                    Task::none()
                }
            },
            Message::Refresh => {
                self.status = "Refreshing...".to_string();
                let root1 = self.xplane_root.clone();
                let root2 = self.xplane_root.clone();
                let root3 = self.xplane_root.clone();
                let root4 = self.xplane_root.clone();
                let root5 = self.xplane_root.clone();

                Task::batch([
                    Task::perform(async move { load_packs(root1) }, Message::SceneryLoaded),
                    Task::perform(async move { load_aircraft(root2) }, Message::AircraftLoaded),
                    Task::perform(
                        async move { load_aircraft_tree(root3) },
                        Message::AircraftTreeLoaded,
                    ),
                    Task::perform(async move { load_plugins(root4) }, Message::PluginsLoaded),
                    Task::perform(async move { load_csls(root5) }, Message::CSLsLoaded),
                ])
            }
            Message::SelectFolder => {
                self.status = "Select X-Plane folder...".to_string();
                Task::perform(
                    async {
                        use native_dialog::FileDialog;
                        FileDialog::new()
                            .set_title("Select X-Plane Folder")
                            .show_open_single_dir()
                            .ok()
                            .flatten()
                    },
                    Message::FolderSelected,
                )
            }
            Message::FolderSelected(path_opt) => {
                if let Some(path) = path_opt {
                    // Validate it's an X-Plane folder
                    match XPlaneManager::new(&path) {
                        Ok(_) => {
                            self.xplane_root = Some(path);
                            self.status = "X-Plane folder set! Reloading...".to_string();
                            // Reload all data
                            let root1 = self.xplane_root.clone();
                            let root2 = self.xplane_root.clone();
                            let root3 = self.xplane_root.clone();
                            return Task::batch([
                                Task::perform(
                                    async move { load_packs(root1) },
                                    Message::SceneryLoaded,
                                ),
                                Task::perform(
                                    async move { load_aircraft(root2) },
                                    Message::AircraftLoaded,
                                ),
                                Task::perform(
                                    async move { load_plugins(root3) },
                                    Message::PluginsLoaded,
                                ),
                            ]);
                        }
                        Err(e) => {
                            self.status = format!("Invalid X-Plane folder: {}", e);
                        }
                    }
                } else {
                    self.status = "Folder selection cancelled".to_string();
                }
                Task::none()
            }
            Message::ToggleAircraftFolder(path) => {
                // Toggle expand/collapse for the folder at path
                if let Some(ref mut tree) = self.aircraft_tree {
                    toggle_folder_at_path(tree, &path);
                }
                Task::none()
            }
            Message::SelectScenery(name) => {
                self.selected_scenery = Some(name.clone());
                if let Some(index) = self.packs.iter().position(|p| p.name == name) {
                    // Fixed card height 75.0 + 10.0 spacing = 85.0 stride
                    let offset = index as f32 * 85.0;
                    return scrollable::scroll_to(
                        self.scenery_scroll_id.clone(),
                        scrollable::AbsoluteOffset { x: 0.0, y: offset },
                    );
                }
                Task::none()
            }
            Message::HoverScenery(name_opt) => {
                if self.hovered_scenery != name_opt {
                    self.hovered_scenery = name_opt;
                }
                Task::none()
            }
            Message::SelectAircraft(path) => {
                self.selected_aircraft = Some(path.clone());

                // Try to find icon11.png or icon.png
                // Based on .acf filename
                let mut icon_handle = None;
                if let Ok(entries) = std::fs::read_dir(&path) {
                    let acf_file = entries
                        .flatten()
                        .into_iter()
                        .find(|e| e.path().extension().map_or(false, |ext| ext == "acf"));

                    if let Some(acf) = acf_file {
                        let acf_stem = acf
                            .path()
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let dir = acf.path().parent().unwrap().to_path_buf();

                        // Look for {acf_stem}_icon11.png or {acf_stem}_icon.png
                        let icon_paths = [
                            dir.join(format!("{}_icon11.png", acf_stem)),
                            dir.join(format!("{}_icon.png", acf_stem)),
                            dir.join("icon11.png"), // fallback
                            dir.join("icon.png"),
                        ];

                        for p in icon_paths {
                            if p.exists() {
                                if let Ok(bytes) = std::fs::read(&p) {
                                    icon_handle = Some(image::Handle::from_bytes(bytes));
                                    break;
                                }
                            }
                        }
                    }
                }
                self.selected_aircraft_icon = icon_handle;
                Task::none()
            }
            Message::SelectPlugin(path) => {
                self.selected_plugin = Some(path);
                Task::none()
            }
            Message::InstallScenery => Task::perform(pick_zip("Scenery"), |p| {
                Message::InstallPicked(Tab::Scenery, p)
            }),
            Message::InstallAircraft => Task::perform(pick_zip("Aircraft"), |p| {
                Message::InstallPicked(Tab::Aircraft, p)
            }),
            Message::InstallPlugin => Task::perform(pick_zip("Plugin"), |p| {
                Message::InstallPicked(Tab::Plugins, p)
            }),
            Message::InstallPicked(tab, path_opt) => {
                if let Some(zip_path) = path_opt {
                    if tab == Tab::Aircraft {
                        let root = self.xplane_root.clone();
                        let aircraft_dir = root.as_ref().map(|r| r.join("Aircraft"));
                        self.status = "Select destination folder under Aircraft...".to_string();
                        return Task::perform(
                            async move { pick_folder("Aircraft Destination", aircraft_dir).await },
                            move |dest_opt| {
                                Message::InstallAircraftDestPicked(zip_path.clone(), dest_opt)
                            },
                        );
                    }

                    let root = self.xplane_root.clone();
                    self.status = format!("Installing to {:?}...", tab);
                    Task::perform(
                        async move { install_addon(root, zip_path, tab, None) },
                        Message::InstallComplete,
                    )
                } else {
                    self.status = "Install cancelled".to_string();
                    Task::none()
                }
            }
            Message::InstallAircraftDestPicked(zip_path, dest_opt) => {
                if let Some(dest_path) = dest_opt {
                    let root = self.xplane_root.clone();
                    self.status = format!("Installing to {}...", dest_path.display());
                    Task::perform(
                        async move { install_addon(root, zip_path, Tab::Aircraft, Some(dest_path)) },
                        Message::InstallComplete,
                    )
                } else {
                    self.status = "Install cancelled (no destination selected)".to_string();
                    Task::none()
                }
            }
            Message::InstallComplete(result) => {
                match result {
                    Ok(name) => {
                        self.status = format!("Installed: {}", name);
                        return Task::done(Message::Refresh);
                    }
                    Err(e) => {
                        self.status = format!("Install error: {}", e);
                    }
                }
                Task::none()
            }
            Message::DeleteAddon(tab) => {
                let has_selection = match tab {
                    Tab::Scenery => self.selected_scenery.is_some(),
                    Tab::Aircraft => self.selected_aircraft.is_some(),
                    Tab::Plugins => self.selected_plugin.is_some(),
                    Tab::CSLs => self.selected_csl.is_some(),
                };
                if has_selection {
                    self.show_delete_confirm = true;
                }
                Task::none()
            }
            Message::ConfirmDelete(tab, confirmed) => {
                self.show_delete_confirm = false;
                if confirmed {
                    let path = match tab {
                        Tab::Scenery => {
                            if let Some(ref name) = self.selected_scenery {
                                self.packs
                                    .iter()
                                    .find(|p| &p.name == name)
                                    .map(|p| p.path.clone())
                            } else {
                                None
                            }
                        }
                        Tab::Aircraft => self.selected_aircraft.clone(),
                        Tab::Plugins => self.selected_plugin.clone(),
                        Tab::CSLs => self.selected_csl.clone(),
                    };

                    if let Some(p) = path {
                        let root = self.xplane_root.clone();
                        self.status = "Deleting...".to_string();
                        return Task::perform(
                            async move { delete_addon(root, p, tab) },
                            |result| match result {
                                Ok(_) => Message::Refresh,
                                Err(e) => Message::SceneryLoaded(Err(e)),
                            },
                        );
                    }
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let navigator = self.view_navigator();
        let content = self.view_xaddonmanager();
        let inspector = self.view_inspector();

        row![
            navigator,
            column![
                content,
                container(inspector)
                    .width(Length::Fill)
                    .height(Length::FillPortion(1))
                    .style(style::container_sidebar)
                    .padding(15)
            ]
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_navigator(&self) -> Element<'_, Message> {
        container(
            column![
                self.sidebar_button("Aircraft", Tab::Aircraft),
                self.sidebar_button("Scenery", Tab::Scenery),
                self.sidebar_button("Plugins", Tab::Plugins),
                self.sidebar_button("CSLs", Tab::CSLs),
            ]
            .spacing(25)
            .padding([20, 0]),
        )
        .width(Length::Fixed(120.0))
        .height(Length::Fill)
        .style(style::container_sidebar)
        .into()
    }

    fn view_xaddonmanager(&self) -> Element<'_, Message> {
        let content = match self.active_tab {
            Tab::Scenery => self.view_scenery(),
            Tab::Aircraft => self.view_aircraft_tree(),
            Tab::Plugins => self.view_addon_list(&self.plugins, "Plugin"),
            Tab::CSLs => self.view_addon_list(&self.csls, "CSL Package"),
        };

        // Path text for display
        let path_text = match &self.xplane_root {
            Some(p) => p.to_string_lossy().to_string(),
            None => "No X-Plane folder".to_string(),
        };

        // Unified Top Bar Action Buttons
        let (install_msg, delete_msg, has_selection) = match self.active_tab {
            Tab::Scenery => (
                Message::InstallScenery,
                Message::DeleteAddon(Tab::Scenery),
                self.selected_scenery.is_some(),
            ),
            Tab::Aircraft => (
                Message::InstallAircraft,
                Message::DeleteAddon(Tab::Aircraft),
                self.selected_aircraft.is_some(),
            ),
            Tab::Plugins => (
                Message::InstallPlugin,
                Message::DeleteAddon(Tab::Plugins),
                self.selected_plugin.is_some(),
            ),
            Tab::CSLs => (
                Message::InstallCSL,
                Message::DeleteAddon(Tab::CSLs),
                self.selected_csl.is_some(),
            ),
        };

        let install_btn = button(
            text("Install...")
                .size(12)
                .align_x(iced::alignment::Horizontal::Center),
        )
        .on_press(install_msg)
        .style(style::button_primary)
        .padding([6, 12]);

        let delete_btn = if has_selection {
            button(
                text("Delete...")
                    .size(12)
                    .align_x(iced::alignment::Horizontal::Center),
            )
            .on_press(delete_msg)
            .style(style::button_secondary)
            .padding([6, 12])
        } else {
            button(
                text("Delete...")
                    .size(12)
                    .align_x(iced::alignment::Horizontal::Center),
            )
            .style(style::button_secondary)
            .padding([6, 12])
        };

        container(
            column![
                // Top Bar
                row![
                    row![install_btn, delete_btn].spacing(10),
                    iced::widget::Space::with_width(Length::Fill),
                    // Right: Path & Set Button
                    row![
                        container(
                            text(path_text)
                                .size(10)
                                .color(style::palette::TEXT_SECONDARY)
                        )
                        .padding([2, 6])
                        .style(|_| container::Style {
                            background: Some(iced::Background::Color(style::palette::SURFACE)),
                            border: iced::Border {
                                radius: 4.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                        button(svg(self.refresh_icon.clone()).width(12).height(12).style(
                            |_, _| svg::Style {
                                color: Some(style::palette::TEXT_SECONDARY),
                            }
                        ),)
                        .on_press(Message::Refresh)
                        .padding([2, 4])
                        .style(style::button_ghost),
                        button(text("Set").size(10))
                            .on_press(Message::SelectFolder)
                            .padding([2, 6])
                            .style(style::button_secondary)
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                ]
                .spacing(20)
                .align_y(iced::Alignment::Center),
                content,
            ]
            .spacing(20)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::FillPortion(2))
        .style(style::container_main_content)
        .into()
    }

    fn view_inspector(&self) -> Element<'_, Message> {
        let map_container = container(responsive(move |size| {
            let zoom = if !self.map_initialized {
                (size.width as f64 / 256.0).log2()
            } else {
                self.map_zoom.max((size.width as f64 / 256.0).log2())
            };

            // Map View
            let map_view = MapView {
                packs: &self.packs,
                selected_scenery: self.selected_scenery.as_ref(),
                hovered_scenery: self.hovered_scenery.as_ref(),
                tile_manager: &self.tile_manager,
                zoom,
                center: self.map_center,
            };

            map_view.into()
        }))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(style::container_card)
        .padding(1)
        .clip(true);

        row![
            map_container,
            container(
                column![
                    text("Inspector Panel").size(18),
                    text("FOLDER:")
                        .size(10)
                        .color(style::palette::TEXT_SECONDARY),
                    text(
                        self.selected_scenery
                            .as_deref()
                            .unwrap_or(self.hovered_scenery.as_deref().unwrap_or("None"))
                    )
                    .size(12),
                    text(
                        if let Some(target_name) = self
                            .selected_scenery
                            .as_ref()
                            .or(self.hovered_scenery.as_ref())
                        {
                            if let Some(pack) = self.packs.iter().find(|p| &p.name == target_name) {
                                format!(
                                    "CATEGORY: {:?} | TILES: {} | AIRPORTS: {}",
                                    pack.category,
                                    pack.tiles.len(),
                                    pack.airports.len()
                                )
                            } else {
                                "".to_string()
                            }
                        } else {
                            "".to_string()
                        }
                    )
                    .size(10),
                ]
                .spacing(10)
            )
            .style(style::container_card)
            .padding(15)
            .width(Length::Fixed(300.0))
            .height(Length::Fill)
        ]
        .spacing(20)
        .height(Length::Fill)
        .into()
    }

    fn sidebar_button(&self, label: &'static str, tab: Tab) -> Element<'_, Message> {
        let is_active = self.active_tab == tab;

        let (icon_handle, active_color) = match tab {
            Tab::Aircraft => (&self.icon_aircraft, Color::WHITE),
            Tab::Scenery => (&self.icon_scenery, Color::from_rgb(0.4, 0.8, 0.4)), // Green
            Tab::Plugins => (&self.icon_plugins, Color::from_rgb(0.4, 0.6, 1.0)), // Blue
            Tab::CSLs => (&self.icon_csls, Color::from_rgb(1.0, 0.6, 0.2)),       // Orange
        };

        let icon = svg(icon_handle.clone())
            .width(Length::Fixed(48.0))
            .height(Length::Fixed(48.0))
            .style(move |_theme, _status| svg::Style {
                color: Some(if is_active {
                    active_color
                } else {
                    style::palette::TEXT_SECONDARY
                }),
            });

        let icon_container = if is_active {
            container(icon).style(move |_| container::Style {
                shadow: iced::Shadow {
                    color: Color::from_rgba(active_color.r, active_color.g, active_color.b, 0.4),
                    offset: iced::Vector::new(0.0, 0.0),
                    blur_radius: 15.0,
                },
                ..Default::default()
            })
        } else {
            container(icon)
        };

        let content = column![
            icon_container,
            text(label)
                .size(14)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .color(if is_active {
                    active_color
                } else {
                    style::palette::TEXT_SECONDARY
                })
        ]
        .spacing(8)
        .align_x(iced::Alignment::Center);

        let style_fn = if is_active {
            style::button_sidebar_active
        } else {
            style::button_sidebar_inactive
        };

        let btn = button(content)
            .on_press(Message::SwitchTab(tab))
            .style(style_fn)
            .padding([15, 0])
            .width(Length::Fill);

        if is_active {
            row![
                btn,
                container(iced::widget::Space::new(
                    Length::Fixed(4.0),
                    Length::Fixed(48.0)
                ))
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(active_color)),
                    border: iced::Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    shadow: iced::Shadow {
                        color: Color::from_rgba(
                            active_color.r,
                            active_color.g,
                            active_color.b,
                            0.8
                        ),
                        offset: iced::Vector::new(0.0, 0.0),
                        blur_radius: 12.0,
                    },
                    ..Default::default()
                })
            ]
            .align_y(iced::Alignment::Center)
            .into()
        } else {
            row![
                btn,
                iced::widget::Space::new(Length::Fixed(4.0), Length::Fixed(48.0))
            ]
            .align_y(iced::Alignment::Center)
            .into()
        }
    }

    fn view_scenery(&self) -> Element<'_, Message> {
        let list = column(
            self.packs
                .iter()
                .map(|pack| self.view_scenery_card(pack))
                .collect::<Vec<_>>(),
        )
        .spacing(10);

        // Overlay confirm if needed (though new design might do it differently, let's keep it simple for now)
        // Ignoring confirm overlay for the card view for this step to keep code clean.

        scrollable(list).id(self.scenery_scroll_id.clone()).into()
    }

    fn view_scenery_card<'a>(&self, pack: &'a SceneryPack) -> Element<'a, Message> {
        let is_active = pack.status == SceneryPackType::Active;
        let is_selected = self.selected_scenery.as_ref() == Some(&pack.name);

        let status_dot = container(iced::widget::Space::new(
            Length::Fixed(0.0),
            Length::Fixed(0.0),
        ))
        .width(Length::Fixed(8.0))
        .height(Length::Fixed(8.0))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(if is_active {
                style::palette::ACCENT_BLUE
            } else {
                style::palette::TEXT_SECONDARY
            })),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        let name_text = text(&pack.name)
            .size(14)
            .color(style::palette::TEXT_PRIMARY);
        let sub_text = text(if is_active { "Active" } else { "Disabled" })
            .size(10)
            .color(style::palette::TEXT_SECONDARY);

        let info_col = column![name_text, sub_text].spacing(4).width(Length::Fill);

        // Type Tag
        // let cat_name = format!("{:?}", pack.category); // Simplified for now
        let tag_color = match pack.category {
            x_adox_core::scenery::SceneryCategory::EarthAirports => style::palette::ACCENT_ORANGE,
            x_adox_core::scenery::SceneryCategory::Library => style::palette::ACCENT_BLUE,
            _ => style::palette::TEXT_SECONDARY,
        };

        // Shorten category for display
        let cat_display = match pack.category {
            x_adox_core::scenery::SceneryCategory::EarthAirports => "AIRPORT",
            x_adox_core::scenery::SceneryCategory::Library => "LIB",
            x_adox_core::scenery::SceneryCategory::EarthScenery => "MESH",
            _ => "SCENERY",
        };

        let type_tag = container(text(cat_display).size(10).color(Color::WHITE))
            .padding([2, 6])
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(tag_color)),
                border: iced::Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        // Action Button
        let action_btn = button(
            text(if is_active { "DISABLE" } else { "ENABLE" })
                .size(10)
                .align_x(iced::alignment::Horizontal::Center),
        )
        .on_press(Message::TogglePack(pack.name.clone()))
        .style(if is_active {
            style::button_secondary
        } else {
            style::button_primary
        })
        .padding([5, 10])
        .width(Length::Fixed(80.0));

        let content_row = row![status_dot, info_col, type_tag, action_btn]
            .spacing(15)
            .align_y(iced::Alignment::Center);

        // Interactive container for selection
        // Interactive container for selection
        button(content_row)
            .on_press(Message::SelectScenery(pack.name.clone()))
            .style(move |theme, status| {
                let mut base = style::button_card(theme, status);
                if is_selected {
                    base.border.color = style::palette::ACCENT_BLUE;
                    base.border.width = 1.0;
                }
                base
            })
            .padding(15)
            .height(Length::Fixed(75.0))
            .width(Length::Fill)
            .into()
    }

    fn view_addon_list<'a>(
        &'a self,
        addons: &'a [DiscoveredAddon],
        label: &str,
    ) -> Element<'a, Message> {
        let is_plugins = label == "Plugin";
        let is_csls = label == "CSL Package";

        let selected_path = if is_plugins {
            &self.selected_plugin
        } else if is_csls {
            &self.selected_csl
        } else {
            &self.selected_aircraft
        };

        let confirm_text = if self.show_delete_confirm
            && ((is_plugins && self.active_tab == Tab::Plugins)
                || (!is_plugins && !is_csls && self.active_tab == Tab::Aircraft)
                || (is_csls && self.active_tab == Tab::CSLs))
        {
            if let Some(ref path) = selected_path {
                Some(format!("Delete {} at '{}'?", label, path.display()))
            } else {
                None
            }
        } else {
            None
        };

        let list_content: Element<'_, Message> = if addons.is_empty() {
            text(format!("No {} found", label)).size(14).into()
        } else {
            let list: Column<Message> =
                addons.iter().fold(Column::new().spacing(4), |col, addon| {
                    let type_label = match &addon.addon_type {
                        AddonType::Aircraft(acf) => format!("• {}", acf),
                        AddonType::Scenery { .. } => "• Scenery".to_string(),
                        AddonType::Plugin { .. } => "• Plugin".to_string(),
                        AddonType::CSL(_) => "• CSL".to_string(),
                    };

                    let is_selected = selected_path.as_ref() == Some(&addon.path);
                    let style = if is_selected {
                        style::button_sidebar_active
                    } else {
                        style::button_sidebar_inactive
                    };

                    let path = addon.path.clone();
                    let row_content: Element<'_, Message> = if is_csls || is_plugins {
                        let is_enabled = addon.is_enabled;
                        let path_for_toggle = path.clone();
                        let _toggle_msg = if is_plugins {
                            Message::TogglePlugin(path_for_toggle.clone(), !is_enabled)
                        } else {
                            Message::ToggleCSL(path_for_toggle.clone(), !is_enabled)
                        };

                        // Selection Message
                        let select_msg = if is_plugins {
                            Message::SelectPlugin(path.clone())
                        } else {
                            Message::SelectCSL(path.clone())
                        };

                        row![
                            checkbox("", is_enabled)
                                .on_toggle(move |e| if is_plugins {
                                    Message::TogglePlugin(path_for_toggle.clone(), e)
                                } else {
                                    Message::ToggleCSL(path_for_toggle.clone(), e)
                                })
                                .text_size(14),
                            button(text(&addon.name).size(14).width(Length::Fill))
                                .on_press(select_msg)
                                .style(style)
                                .padding([4, 8])
                                .width(Length::Fill),
                        ]
                        .spacing(5)
                        .into()
                    } else {
                        let addon_btn = button(
                            row![
                                text(&addon.name).size(14).width(Length::Fill),
                                text(type_label)
                                    .size(12)
                                    .color(Color::from_rgb(0.6, 0.6, 0.6)),
                            ]
                            .spacing(10)
                            .align_y(iced::Alignment::Center),
                        )
                        .on_press(if is_plugins {
                            Message::SelectPlugin(path)
                        } else {
                            Message::SelectAircraft(path)
                        })
                        .style(style)
                        .padding([4, 8])
                        .width(Length::Fill);

                        addon_btn.into()
                    };

                    col.push(row_content)
                });

            Element::from(scrollable(list).height(Length::Fill))
        };

        let main_content = column![list_content].spacing(10);

        if let Some(confirm_msg) = confirm_text {
            column![
                main_content,
                row![
                    text(confirm_msg).size(14),
                    button("Yes, Delete")
                        .on_press(Message::ConfirmDelete(
                            if is_plugins {
                                Tab::Plugins
                            } else if is_csls {
                                Tab::CSLs
                            } else {
                                Tab::Aircraft
                            },
                            true
                        ))
                        .padding([6, 12]),
                    button("Cancel")
                        .on_press(Message::ConfirmDelete(
                            if is_plugins {
                                Tab::Plugins
                            } else if is_csls {
                                Tab::CSLs
                            } else {
                                Tab::Aircraft
                            },
                            false
                        ))
                        .padding([6, 12]),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center)
            ]
            .spacing(10)
            .into()
        } else {
            main_content.into()
        }
    }

    fn view_aircraft_tree(&self) -> Element<'_, Message> {
        let confirm_text = if self.show_delete_confirm && self.active_tab == Tab::Aircraft {
            if let Some(ref path) = self.selected_aircraft {
                Some(format!("Delete aircraft at '{}'?", path.display()))
            } else {
                None
            }
        } else {
            None
        };

        let tree_content: Element<'_, Message> = match &self.aircraft_tree {
            Some(tree) => {
                let items = self.collect_tree_nodes(tree, 0);
                let col: Column<Message> = items
                    .into_iter()
                    .fold(Column::new().spacing(2), |c, e| c.push(e));
                Element::from(scrollable(col).height(Length::Fill))
            }
            None => text("Loading aircraft...").size(14).into(),
        };

        let preview: Element<'_, Message> = if let Some(icon) = &self.selected_aircraft_icon {
            container(iced::widget::image(icon.clone()))
                .width(Length::FillPortion(1))
                .height(Length::Fill)
                .padding(20)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(style::container_card)
                .into()
        } else {
            container(text("No preview available").color(style::palette::TEXT_SECONDARY))
                .width(Length::FillPortion(1))
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(style::container_card)
                .into()
        };

        let main_content = row![
            container(tree_content).width(Length::FillPortion(2)),
            preview
        ]
        .spacing(20);

        if let Some(confirm_msg) = confirm_text {
            column![
                main_content,
                row![
                    text(confirm_msg).size(14),
                    button("Yes, Delete")
                        .on_press(Message::ConfirmDelete(Tab::Aircraft, true))
                        .padding([6, 12]),
                    button("Cancel")
                        .on_press(Message::ConfirmDelete(Tab::Aircraft, false))
                        .padding([6, 12]),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center)
            ]
            .spacing(10)
            .into()
        } else {
            main_content.into()
        }
    }

    fn collect_tree_nodes(&self, node: &AircraftNode, depth: usize) -> Vec<Element<'_, Message>> {
        let mut result = Vec::new();
        let indent = 20 * depth;

        // Determine icon based on node type
        let (icon, label_color) = if node.is_folder {
            let arrow = if node.is_expanded { "v" } else { ">" };
            (arrow.to_string(), Color::WHITE)
        } else if node.acf_file.is_some() {
            ("   •".to_string(), Color::from_rgb(0.6, 0.9, 0.6))
        } else {
            ("   -".to_string(), Color::from_rgb(0.6, 0.6, 0.6))
        };

        let display_name = if let Some(acf) = &node.acf_file {
            format!("{} ({})", node.name, acf)
        } else {
            node.name.clone()
        };

        let is_selected = self.selected_aircraft.as_ref() == Some(&node.path);
        let style = if is_selected {
            button::primary
        } else {
            style::button_ghost
        };

        let node_row: Element<'_, Message> = if node.is_folder {
            let path = node.path.clone();
            let path_for_select = node.path.clone();

            row![
                button(text(icon.clone()).size(14))
                    .on_press(Message::ToggleAircraftFolder(path))
                    .padding([4, 8])
                    .style(style::button_ghost),
                button(text(display_name.clone()).size(14).color(label_color))
                    .on_press(Message::SelectAircraft(path_for_select))
                    .style(style)
                    .padding([4, 8])
            ]
            .spacing(5)
            .into()
        } else {
            let path = node.path.clone();
            button(
                row![
                    text(icon).size(12),
                    text(display_name).size(14).color(label_color),
                ]
                .spacing(5),
            )
            .on_press(Message::SelectAircraft(path))
            .style(style)
            .padding([4, 8])
            .into()
        };

        let indented: Element<'_, Message> = row![
            container(text("")).width(Length::Fixed(indent as f32)),
            node_row,
        ]
        .into();

        result.push(indented);

        // Collect children if expanded
        if node.is_expanded {
            for child in &node.children {
                result.extend(self.collect_tree_nodes(child, depth + 1));
            }
        }

        result
    }
}

// Data loading functions
fn load_packs(root: Option<PathBuf>) -> Result<Vec<SceneryPack>, String> {
    let root = root.ok_or("X-Plane root not found")?;
    let xpm = XPlaneManager::new(&root).map_err(|e| e.to_string())?;
    let mut sm = SceneryManager::new(xpm.get_scenery_packs_path());
    sm.load().map_err(|e| e.to_string())?;
    Ok(sm.packs)
}

fn toggle_plugin(root: Option<PathBuf>, path: PathBuf, enable: bool) -> Result<(), String> {
    let root = root.ok_or("X-Plane root not found")?;
    ModManager::set_plugin_enabled(&root, &path, enable).map_err(|e| e.to_string())?;
    Ok(())
}

fn load_aircraft(root: Option<PathBuf>) -> Result<Vec<DiscoveredAddon>, String> {
    let root = root.ok_or("X-Plane root not found")?;
    let aircraft_path = root.join("Aircraft");
    Ok(DiscoveryManager::scan_aircraft(&aircraft_path))
}

fn load_plugins(root: Option<PathBuf>) -> Result<Vec<DiscoveredAddon>, String> {
    let root = root.ok_or("X-Plane root not found")?;
    Ok(DiscoveryManager::scan_plugins(&root))
}

fn load_csls(root: Option<PathBuf>) -> Result<Vec<DiscoveredAddon>, String> {
    let root = root.ok_or("X-Plane root not found")?;
    Ok(DiscoveryManager::scan_csls(&root))
}

fn toggle_csl(root: Option<PathBuf>, path: PathBuf, enable: bool) -> Result<(), String> {
    let _root = root.ok_or("X-Plane root not found")?;
    let name = path.file_name().ok_or("Invalid CSL path")?;
    let parent = path.parent().ok_or("Invalid CSL parent")?;
    let grandparent = parent.parent().ok_or("Invalid CSL grandparent")?;

    let dest_parent = if enable {
        grandparent.join("CSL")
    } else {
        grandparent.join("CSL (disabled)")
    };

    if !dest_parent.exists() {
        std::fs::create_dir_all(&dest_parent).map_err(|e| e.to_string())?;
    }

    let dest_path = dest_parent.join(name);
    std::fs::rename(path, dest_path).map_err(|e| e.to_string())?;

    Ok(())
}

fn toggle_pack(root: Option<PathBuf>, name: String, enable: bool) -> Result<(), String> {
    let root = root.ok_or("X-Plane root not found")?;
    let xpm = XPlaneManager::new(&root).map_err(|e| e.to_string())?;
    let mut sm = SceneryManager::new(xpm.get_scenery_packs_path());
    sm.load().map_err(|e| e.to_string())?;

    if enable {
        sm.enable_pack(&name);
    } else {
        sm.disable_pack(&name);
    }

    sm.save().map_err(|e| e.to_string())
}

fn load_aircraft_tree(root: Option<PathBuf>) -> Result<AircraftNode, String> {
    let root = root.ok_or("X-Plane root not found")?;
    let aircraft_path = root.join("Aircraft");

    if !aircraft_path.exists() {
        return Err("Aircraft folder not found".to_string());
    }

    Ok(build_aircraft_tree(&aircraft_path))
}

fn build_aircraft_tree(path: &std::path::Path) -> AircraftNode {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Aircraft".to_string());

    let mut children = Vec::new();
    let mut acf_file = None;

    if let Ok(entries) = std::fs::read_dir(path) {
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        for entry in entries {
            let entry_path = entry.path();
            let entry_name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files
            if entry_name.starts_with('.') {
                continue;
            }

            if entry_path.is_dir() {
                // Recursively build tree for subdirectories
                children.push(build_aircraft_tree(&entry_path));
            } else if entry_name.ends_with(".acf") {
                // Found an aircraft file
                acf_file = Some(entry_name);
            }
        }
    }

    AircraftNode {
        name,
        path: path.to_path_buf(),
        is_folder: acf_file.is_none() && !children.is_empty(),
        is_expanded: path.file_name().map(|n| n == "Aircraft").unwrap_or(false), // Expand root
        children,
        acf_file,
    }
}

fn toggle_folder_at_path(node: &mut AircraftNode, target_path: &std::path::Path) {
    if node.path == target_path {
        node.is_expanded = !node.is_expanded;
        return;
    }

    for child in &mut node.children {
        toggle_folder_at_path(child, target_path);
    }
}

fn install_addon(
    root: Option<PathBuf>,
    zip_path: PathBuf,
    tab: Tab,
    dest_override: Option<PathBuf>,
) -> Result<String, String> {
    use std::io::{Read, Write};

    let root = root.ok_or("X-Plane root not found")?;
    let dest_dir = if let Some(dest) = dest_override {
        dest
    } else {
        match tab {
            Tab::Scenery => root.join("Custom Scenery"),
            Tab::Aircraft => root.join("Aircraft"),
            Tab::Plugins => root.join("Resources").join("plugins"),
            Tab::CSLs => {
                let ivap = root
                    .join("Resources")
                    .join("plugins")
                    .join("X-Ivap Resources")
                    .join("CSL");
                let xpilot = root
                    .join("Resources")
                    .join("plugins")
                    .join("xPilot")
                    .join("Resources")
                    .join("CSL");
                let custom = root.join("Custom Data").join("CSL");

                if xpilot.exists() {
                    xpilot
                } else if custom.exists() {
                    custom
                } else if ivap.exists() {
                    ivap
                } else {
                    custom
                }
            }
        }
    };

    if !dest_dir.exists() {
        return Err(format!(
            "Destination directory {} not found",
            dest_dir.display()
        ));
    }

    // Open the zip file
    let file = std::fs::File::open(&zip_path).map_err(|e| format!("Failed to open zip: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip: {}", e))?;

    // Determine the top-level folder name from the zip
    let top_folder = if let Some(first) = archive.file_names().next() {
        first.split('/').next().unwrap_or("Unknown").to_string()
    } else {
        return Err("Empty zip archive".to_string());
    };

    // Extract to destination
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {}", e))?;

        let outpath = dest_dir.join(file.name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create dir: {}", e))?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent dir: {}", e))?;
            }
            let mut outfile = std::fs::File::create(&outpath)
                .map_err(|e| format!("Failed to create file: {}", e))?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .map_err(|e| format!("Failed to read zip content: {}", e))?;
            outfile
                .write_all(&buffer)
                .map_err(|e| format!("Failed to write file: {}", e))?;
        }
    }

    // Special handling for Scenery: add to scenery_packs.ini
    if matches!(tab, Tab::Scenery) {
        let xpm = XPlaneManager::new(&root).map_err(|e| e.to_string())?;
        let mut sm = SceneryManager::new(xpm.get_scenery_packs_path());
        let _ = sm.load(); // Ignore load error if file doesn't exist yet

        // Add the new pack at the top of the list
        sm.packs.insert(
            0,
            x_adox_core::scenery::SceneryPack {
                name: top_folder.clone(),
                path: PathBuf::from(format!("Custom Scenery/{}/", top_folder)),
                status: x_adox_core::scenery::SceneryPackType::Active,
                category: x_adox_core::scenery::SceneryCategory::default(),
                airports: Vec::new(),
                tiles: Vec::new(),
            },
        );

        sm.save().map_err(|e| e.to_string())?;
    }

    Ok(top_folder)
}

fn delete_addon(root: Option<PathBuf>, path: PathBuf, tab: Tab) -> Result<(), String> {
    let root = root.ok_or("X-Plane root not found")?;

    // Resolve the path
    let full_path = if path.is_relative() {
        root.join(&path)
    } else {
        path.clone()
    };

    // Safety check - make sure we're deleting from the right folder
    let is_csl = tab == Tab::CSLs;

    if is_csl {
        // CSL can be in CSL or CSL (disabled) under any of the standard CSL roots
        let csl_roots = [
            root.join("Resources")
                .join("plugins")
                .join("X-Ivap Resources"),
            root.join("Resources")
                .join("plugins")
                .join("xPilot")
                .join("Resources"),
            root.join("Custom Data"),
        ];

        let mut allowed = false;
        for csl_root in csl_roots {
            let csl_enabled = csl_root.join("CSL");
            let csl_disabled = csl_root.join("CSL (disabled)");
            if full_path.starts_with(&csl_enabled) || full_path.starts_with(&csl_disabled) {
                allowed = true;
                break;
            }
        }

        if !allowed {
            return Err(format!(
                "Safety check failed: {} is not inside CSL folders",
                full_path.display()
            ));
        }
    } else {
        let allowed_dir = match tab {
            Tab::Scenery => root.join("Custom Scenery"),
            Tab::Aircraft => root.join("Aircraft"),
            Tab::Plugins => root.join("Resources").join("plugins"),
            Tab::CSLs => unreachable!(), // Handled above
        };

        if !full_path.starts_with(&allowed_dir) {
            return Err(format!(
                "Safety check failed: {} is not inside {}",
                full_path.display(),
                allowed_dir.display()
            ));
        }
    }

    // Delete the folder/file
    if full_path.exists() {
        if full_path.is_dir() {
            std::fs::remove_dir_all(&full_path)
                .map_err(|e| format!("Failed to delete dir: {}", e))?;
        } else {
            std::fs::remove_file(&full_path)
                .map_err(|e| format!("Failed to delete file: {}", e))?;
        }
    }

    // Special handling for Scenery: remove from scenery_packs.ini
    if matches!(tab, Tab::Scenery) {
        let xpm = XPlaneManager::new(&root).map_err(|e| e.to_string())?;
        let mut sm = SceneryManager::new(xpm.get_scenery_packs_path());
        let _ = sm.load();

        sm.packs.retain(|p| p.path != path);
        sm.save().map_err(|e| e.to_string())?;
    }

    Ok(())
}

async fn pick_zip(label: &str) -> Option<PathBuf> {
    use native_dialog::FileDialog;
    FileDialog::new()
        .set_title(&format!("Select {} Package (.zip)", label))
        .add_filter("ZIP Archive", &["zip"])
        .show_open_single_file()
        .ok()
        .flatten()
}

async fn pick_folder(title: &str, start_dir: Option<PathBuf>) -> Option<PathBuf> {
    use native_dialog::FileDialog;
    let dialog = FileDialog::new().set_title(title);
    match start_dir {
        Some(path) => dialog.set_location(&path).show_open_single_dir(),
        None => dialog.show_open_single_dir(),
    }
    .ok()
    .flatten()
}

// --- Slippy Map / Mercator Math ---
// --- Slippy Map / Mercator Math ---
const TILE_SIZE: f64 = 256.0;

fn lon_to_x(lon: f64, zoom: f64) -> f64 {
    ((lon + 180.0) / 360.0) * 2.0f64.powf(zoom) * TILE_SIZE
}

fn lat_to_y(lat: f64, zoom: f64) -> f64 {
    let lat_rad = lat.to_radians();
    (1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0
        * 2.0f64.powf(zoom)
        * TILE_SIZE
}

fn x_to_lon(x: f64, zoom: f64) -> f64 {
    (x / (TILE_SIZE * 2.0f64.powf(zoom))) * 360.0 - 180.0
}

fn y_to_lat(y: f64, zoom: f64) -> f64 {
    let n = std::f64::consts::PI - 2.0 * std::f64::consts::PI * y / (TILE_SIZE * 2.0f64.powf(zoom));
    (0.5 * (n.exp() - (-n).exp())).atan().to_degrees()
}

// --- Tile Management ---
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TileCoords {
    x: u32,
    y: u32,
    z: u32,
}

impl TileCoords {
    fn url(&self) -> String {
        format!(
            "https://tile.openstreetmap.org/{}/{}/{}.png",
            self.z, self.x, self.y
        )
    }
}

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

struct TileManager {
    tiles: Arc<Mutex<LruCache<TileCoords, image::Handle>>>,
    pending: Arc<Mutex<std::collections::HashSet<TileCoords>>>,
}

impl TileManager {
    fn new() -> Self {
        Self {
            tiles: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(300).unwrap()))),
            pending: Arc::new(Mutex::new(std::collections::HashSet::new())),
        }
    }

    fn get_tile(&self, coords: TileCoords) -> Option<image::Handle> {
        let mut tiles = self.tiles.lock().unwrap();
        tiles.get(&coords).cloned()
    }

    fn request_tile(&self, coords: TileCoords) {
        {
            let mut pending = self.pending.lock().unwrap();
            if pending.contains(&coords) {
                return;
            }
            let tiles = self.tiles.lock().unwrap();
            if tiles.contains(&coords) {
                return;
            }
            pending.insert(coords);
        }

        let tiles_arc = Arc::clone(&self.tiles);
        let pending_arc = Arc::clone(&self.pending);

        // Simple background fetcher using std::thread to avoid tokio runtime dependency
        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .user_agent("X-Addon-Oxide/0.1.0")
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap();

            match client.get(coords.url()).send() {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        eprintln!("Tile fetch failed for {:?}: Status {}", coords, status);
                    } else if let Ok(bytes) = resp.bytes() {
                        let handle = image::Handle::from_bytes(bytes.to_vec());
                        let mut tiles = tiles_arc.lock().unwrap();
                        tiles.put(coords, handle);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to fetch tile {:?}: {}", coords, e);
                }
            }
            let mut pending = pending_arc.lock().unwrap();
            pending.remove(&coords);
        });
    }
}

struct MapView<'a> {
    packs: &'a [SceneryPack],
    selected_scenery: Option<&'a String>,
    hovered_scenery: Option<&'a String>,
    tile_manager: &'a TileManager,
    zoom: f64,          // Fractional zoom (e.g., 2.5)
    center: (f64, f64), // (Lat, Lon)
}

#[derive(Debug, Clone, Copy, Default)]
struct MapState {
    is_dragging: bool,
    last_cursor: Option<iced::Point>,
}

impl<'a, Theme, Renderer> Widget<Message, Theme, Renderer> for MapView<'a>
where
    Renderer: renderer::Renderer + advanced::image::Renderer<Handle = image::Handle>,
{
    fn size(&self) -> iced::Size<Length> {
        iced::Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<MapState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(MapState::default())
    }

    fn layout(
        &self,
        _tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max())
    }

    fn draw(
        &self,
        _tree: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let zoom = self.zoom;
        let (center_lat, center_lon) = self.center;
        let zoom_scale = 2.0f64.powf(zoom);

        let camera_center_x = lon_to_x(center_lon, 0.0);
        let camera_center_y = lat_to_y(center_lat, 0.0);

        renderer.with_layer(bounds, |renderer| {
            // Background fill
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: iced::Border::default(),
                    ..Default::default()
                },
                Color::from_rgb(0.05, 0.05, 0.05),
            );

            // --- Tile Layer ---
            let z = zoom.floor().clamp(0.0, 19.0) as u32;
            let num_tiles = 2u32.pow(z);
            let tile_size_z0 = TILE_SIZE / 2.0f64.powf(z as f64);

            let half_w = (bounds.width as f64 / 2.0) / zoom_scale;
            let half_h = (bounds.height as f64 / 2.0) / zoom_scale;

            let view_left = camera_center_x - half_w;
            let view_right = camera_center_x + half_w;
            let view_top = camera_center_y - half_h;
            let view_bottom = camera_center_y + half_h;

            let min_tx = (view_left / tile_size_z0).floor() as i32;
            let max_tx = (view_right / tile_size_z0).ceil() as i32;
            let min_ty = (view_top / tile_size_z0).floor() as i32;
            let max_ty = (view_bottom / tile_size_z0).ceil() as i32;

            for tx in min_tx..=max_tx {
                if tx < 0 || tx >= num_tiles as i32 {
                    continue;
                }
                for ty in min_ty..=max_ty {
                    if ty < 0 || ty >= num_tiles as i32 {
                        continue;
                    }

                    let coords = TileCoords {
                        x: tx as u32,
                        y: ty as u32,
                        z,
                    };
                    let tile_world_x = tx as f64 * tile_size_z0;
                    let tile_world_y = ty as f64 * tile_size_z0;

                    let screen_x = bounds.x
                        + (bounds.width / 2.0)
                        + ((tile_world_x - camera_center_x) * zoom_scale) as f32;
                    let screen_y = bounds.y
                        + (bounds.height / 2.0)
                        + ((tile_world_y - camera_center_y) * zoom_scale) as f32;
                    let current_tile_size = (tile_size_z0 * zoom_scale) as f32;

                    let tile_rect = Rectangle {
                        x: screen_x,
                        y: screen_y,
                        width: current_tile_size,
                        height: current_tile_size,
                    };

                    if let Some(handle) = self.tile_manager.get_tile(coords) {
                        renderer.draw_image(
                            advanced::image::Image {
                                handle,
                                filter_method: image::FilterMethod::Linear,
                                rotation: Radians(0.0),
                                opacity: 1.0,
                                snap: false,
                            },
                            tile_rect,
                        );
                    } else {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: tile_rect,
                                ..Default::default()
                            },
                            Color::from_rgb(0.1, 0.1, 0.1),
                        );
                        self.tile_manager.request_tile(coords);
                    }
                }
            }
        });

        // --- Marker Layer ---
        // Draw markers in a separate layer on top to ensure visibility
        renderer.with_layer(bounds, |renderer| {
            let square_size = 4.0;
            let selected_size = 8.0;

            for pack in self.packs {
                let is_selected = self.selected_scenery == Some(&pack.name);
                let is_hovered = self.hovered_scenery == Some(&pack.name);
                let base_color = match pack.status {
                    SceneryPackType::Active => Color::from_rgb(0.0, 1.0, 0.0),
                    SceneryPackType::Disabled => Color::from_rgb(1.0, 0.0, 0.0),
                };
                let fill_color = if is_selected || is_hovered {
                    Color::from_rgb(1.0, 1.0, 0.0)
                } else {
                    base_color
                };
                let size = if is_selected || is_hovered {
                    selected_size
                } else {
                    square_size
                };
                let half_size = size / 2.0;

                if pack.airports.is_empty() {
                    for &(lat, lon) in &pack.tiles {
                        let wx = lon_to_x(lon as f64 + 0.5, 0.0);
                        let wy = lat_to_y(lat as f64 + 0.5, 0.0);

                        let sx = bounds.x
                            + (bounds.width / 2.0)
                            + ((wx - camera_center_x) * zoom_scale) as f32;
                        let sy = bounds.y
                            + (bounds.height / 2.0)
                            + ((wy - camera_center_y) * zoom_scale) as f32;

                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: Rectangle {
                                    x: sx - half_size,
                                    y: sy - half_size,
                                    width: size,
                                    height: size,
                                },
                                border: iced::Border {
                                    color: Color::BLACK,
                                    width: 1.0,
                                    radius: (size / 4.0).into(),
                                },
                                ..Default::default()
                            },
                            fill_color,
                        );
                    }
                }
                for airport in &pack.airports {
                    if let (Some(lat), Some(lon)) = (airport.lat, airport.lon) {
                        let wx = lon_to_x(lon as f64, 0.0);
                        let wy = lat_to_y(lat as f64, 0.0);

                        let sx = bounds.x
                            + (bounds.width / 2.0)
                            + ((wx - camera_center_x) * zoom_scale) as f32;
                        let sy = bounds.y
                            + (bounds.height / 2.0)
                            + ((wy - camera_center_y) * zoom_scale) as f32;

                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: Rectangle {
                                    x: sx - half_size,
                                    y: sy - half_size,
                                    width: size,
                                    height: size,
                                },
                                border: iced::Border {
                                    color: Color::BLACK,
                                    width: 1.0,
                                    radius: (size / 4.0).into(),
                                },
                                ..Default::default()
                            },
                            fill_color,
                        );
                    }
                }
            }
        });
    }

    fn on_event(
        &mut self,
        tree: &mut widget::Tree,
        event: Event,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> advanced::graphics::core::event::Status {
        let state = tree.state.downcast_mut::<MapState>();
        let bounds = layout.bounds();
        let zoom = self.zoom;
        let (center_lat, center_lon) = self.center;

        let camera_x = lon_to_x(center_lon, 0.0);
        let camera_y = lat_to_y(center_lat, 0.0);
        let scale = 2.0f64.powf(zoom);

        let cursor_point = cursor.position_in(bounds);
        let mouse_z0 = cursor_point.map(|p| {
            let rx = (p.x as f64) - (bounds.width as f64 / 2.0);
            let ry = (p.y as f64) - (bounds.height as f64 / 2.0);
            (camera_x + rx / scale, camera_y + ry / scale)
        });

        let coords = mouse_z0.and_then(|(wx, wy)| {
            let lon = x_to_lon(wx, 0.0);
            let lat = y_to_lat(wy, 0.0);

            if lon >= -180.0 && lon <= 180.0 && lat >= -85.0511 && lat <= 85.0511 {
                Some((lat, lon))
            } else {
                None
            }
        });

        match event {
            Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
                if let Some(p) = cursor_point {
                    let d = match delta {
                        iced::mouse::ScrollDelta::Lines { y, .. } => y as f64,
                        iced::mouse::ScrollDelta::Pixels { y, .. } => (y as f64) / 100.0,
                    };
                    let min_zoom = (bounds.width as f64 / TILE_SIZE).log2();
                    let new_zoom = (zoom + d * 0.2).clamp(min_zoom, 19.0);

                    if (new_zoom - zoom).abs() > 0.001 {
                        let new_scale = 2.0f64.powf(new_zoom);

                        let mx = (p.x as f64) - (bounds.width as f64 / 2.0);
                        let my = (p.y as f64) - (bounds.height as f64 / 2.0);

                        let new_camera_x = camera_x + mx / scale - mx / new_scale;
                        let new_camera_y = camera_y + my / scale - my / new_scale;

                        let new_half_w = (bounds.width as f64 / 2.0) / new_scale;
                        let new_camera_x_clamped =
                            new_camera_x.clamp(new_half_w, TILE_SIZE - new_half_w);
                        let new_camera_y_clamped = new_camera_y.clamp(0.0, TILE_SIZE);

                        shell.publish(Message::MapZoom {
                            new_center: (
                                y_to_lat(new_camera_y_clamped, 0.0),
                                x_to_lon(new_camera_x_clamped, 0.0),
                            ),
                            new_zoom,
                        });
                        return advanced::graphics::core::event::Status::Captured;
                    }
                }
            }
            Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                if let Some(coords) = coords {
                    for pack in self.packs {
                        if pack.airports.is_empty() {
                            for &(lat, lon) in &pack.tiles {
                                if (lat as f64 + 0.5 - coords.0).abs() < 0.5
                                    && (lon as f64 + 0.5 - coords.1).abs() < 0.5
                                {
                                    shell.publish(Message::SelectScenery(pack.name.clone()));
                                    return advanced::graphics::core::event::Status::Captured;
                                }
                            }
                        }
                        for airport in &pack.airports {
                            if let (Some(lat), Some(lon), (wx, wy)) =
                                (airport.lat, airport.lon, mouse_z0.unwrap())
                            {
                                let tx = lon_to_x(lon as f64, 0.0);
                                let ty = lat_to_y(lat as f64, 0.0);
                                let dist_sq = (tx - wx).powi(2) + (ty - wy).powi(2);

                                // Use a 10px hit radius in screen pixels
                                if dist_sq < (10.0 / scale).powi(2) {
                                    shell.publish(Message::SelectScenery(pack.name.clone()));
                                    return advanced::graphics::core::event::Status::Captured;
                                }
                            }
                        }
                    }
                }

                // Start dragging if no selection was made
                // Use cursor.position() (global coords) to match CursorMoved event
                if cursor.is_over(bounds) {
                    if let Some(position) = cursor.position() {
                        state.is_dragging = true;
                        state.last_cursor = Some(position);
                        return advanced::graphics::core::event::Status::Captured;
                    }
                }
            }
            Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                if state.is_dragging {
                    state.is_dragging = false;
                    state.last_cursor = None;
                    return advanced::graphics::core::event::Status::Captured;
                }
            }
            Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                if state.is_dragging {
                    if let Some(last_pos) = state.last_cursor {
                        let delta = position - last_pos;
                        state.last_cursor = Some(position);

                        let dx = delta.x as f64 / scale;
                        let dy = delta.y as f64 / scale;

                        // Calculate new center in world pixels (zoom 0)
                        let new_wx = camera_x - dx;
                        let new_wy = camera_y - dy;

                        // Calculate constraints (same logic as MapZoom clamping)
                        let half_vw = (bounds.width as f64 / 2.0) / scale;
                        let half_vh = (bounds.height as f64 / 2.0) / scale;

                        // Clamp X
                        let clamped_wx = if half_vw * 2.0 >= TILE_SIZE {
                            TILE_SIZE / 2.0 // Center if viewport >= world
                        } else {
                            new_wx.clamp(half_vw, TILE_SIZE - half_vw)
                        };

                        // Clamp Y
                        let clamped_wy = if half_vh * 2.0 >= TILE_SIZE {
                            TILE_SIZE / 2.0 // Center if viewport >= world
                        } else {
                            new_wy.clamp(half_vh, TILE_SIZE - half_vh)
                        };

                        shell.publish(Message::MapZoom {
                            new_center: (y_to_lat(clamped_wy, 0.0), x_to_lon(clamped_wx, 0.0)),
                            new_zoom: zoom,
                        });
                        return advanced::graphics::core::event::Status::Captured;
                    }
                }

                if let Some(coords) = coords {
                    for pack in self.packs {
                        if pack.airports.is_empty() {
                            for &(lat, lon) in &pack.tiles {
                                if (lat as f64 + 0.5 - coords.0).abs() < 0.5
                                    && (lon as f64 + 0.5 - coords.1).abs() < 0.5
                                {
                                    if self.hovered_scenery != Some(&pack.name) {
                                        shell.publish(Message::HoverScenery(Some(
                                            pack.name.clone(),
                                        )));
                                    }
                                    return advanced::graphics::core::event::Status::Captured;
                                }
                            }
                        }
                        for airport in &pack.airports {
                            if let (Some(lat), Some(lon), (wx, wy)) =
                                (airport.lat, airport.lon, mouse_z0.unwrap())
                            {
                                let tx = lon_to_x(lon as f64, 0.0);
                                let ty = lat_to_y(lat as f64, 0.0);
                                let dist_sq = (tx - wx).powi(2) + (ty - wy).powi(2);

                                // Use a 10px hit radius in screen pixels
                                if dist_sq < (10.0 / scale).powi(2) {
                                    if self.hovered_scenery != Some(&pack.name) {
                                        shell.publish(Message::HoverScenery(Some(
                                            pack.name.clone(),
                                        )));
                                    }
                                    return advanced::graphics::core::event::Status::Captured;
                                }
                            }
                        }
                    }
                    if self.hovered_scenery.is_some() {
                        shell.publish(Message::HoverScenery(None));
                        return advanced::graphics::core::event::Status::Captured;
                    }
                }
            }
            _ => {}
        }
        advanced::graphics::core::event::Status::Ignored
    }

    fn mouse_interaction(
        &self,
        _tree: &widget::Tree,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

impl<'a, Theme, Renderer> From<MapView<'a>> for Element<'a, Message, Theme, Renderer>
where
    Theme: 'a,
    Renderer: 'a + renderer::Renderer + advanced::image::Renderer<Handle = image::Handle>,
{
    fn from(map_view: MapView<'a>) -> Self {
        Self::new(map_view)
    }
}
