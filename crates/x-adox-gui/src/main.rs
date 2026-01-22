use iced::widget::{
    button, checkbox, column, container, image, pick_list, progress_bar, responsive, row,
    scrollable, slider, stack, svg, text, text_editor, text_input, tooltip, Column, Row,
};
use iced::window::icon;
use iced::{Background, Border, Color, Element, Length, Renderer, Shadow, Task, Theme};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use x_adox_bitnet::BitNetModel;
use x_adox_core::discovery::{AddonType, DiscoveredAddon, DiscoveryManager};
use x_adox_core::management::ModManager;
use x_adox_core::profiles::{Profile, ProfileCollection, ProfileManager};
use x_adox_core::scenery::{SceneryCategory, SceneryManager, SceneryPack, SceneryPackType};
use x_adox_core::XPlaneManager;

mod map;
mod style;
use map::{MapView, TileManager};

const AIRCRAFT_CATEGORIES: &[&str] = &[
    "Airliner",
    "General Aviation",
    "Military",
    "Helicopter",
    "Glider",
    "Business Jet",
];

const MANUFACTURERS: &[&str] = &[
    "Airbus",
    "Antonov",
    "Beechcraft",
    "Boeing",
    "Bombardier",
    "Cessna",
    "Cirrus",
    "De Havilland",
    "Diamond",
    "Embraer",
    "Flight Design",
    "Fokker",
    "Gulfstream",
    "Icon",
    "Ilyushin",
    "Lockheed",
    "McDonnell Douglas",
    "Mooney",
    "Pilatus",
    "Piper",
    "Robin",
    "Socata",
    "Tupolev",
    "Van's",
];

fn main() -> iced::Result {
    let icon_data = include_bytes!("../../../icon.png");
    let window_icon = icon::from_file_data(icon_data, None).ok();

    iced::application("X-Addon-Oxide", App::update, App::view)
        .theme(|_| Theme::Dark)
        .window(iced::window::Settings {
            icon: window_icon,
            ..Default::default()
        })
        .run_with(App::new)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Scenery,
    Aircraft,
    Plugins,
    CSLs,
    Heuristics,
    Issues,
    Utilities,
    Settings,
}

#[derive(Debug, Clone, Hash, PartialEq)]
struct AircraftNode {
    name: String,
    path: PathBuf,
    is_folder: bool,
    is_expanded: bool,
    children: Vec<AircraftNode>,
    acf_file: Option<String>, // .acf filename if aircraft
    is_enabled: bool,
    tags: Vec<String>,
}

#[derive(Debug, Clone)]
enum Message {
    // Tab navigation
    SwitchTab(Tab),

    // Scenery
    SceneryLoaded(Result<Arc<Vec<SceneryPack>>, String>),
    TogglePack(String),
    PackToggled(Result<(), String>),

    // Aircraft & Plugins
    AircraftLoaded(Result<Arc<Vec<DiscoveredAddon>>, String>),
    ToggleAircraft(PathBuf, bool),
    AircraftToggled(Result<(), String>),
    PluginsLoaded(Result<Arc<Vec<DiscoveredAddon>>, String>),
    TogglePlugin(PathBuf, bool),
    PluginToggled(Result<(), String>),
    CSLsLoaded(Result<Arc<Vec<DiscoveredAddon>>, String>),
    ToggleCSL(PathBuf, bool),

    // Common
    Refresh,
    SelectFolder,
    FolderSelected(Option<PathBuf>),
    SelectXPlaneRoot(PathBuf),

    // Aircraft tree
    ToggleAircraftFolder(PathBuf),
    AircraftTreeLoaded(Result<Arc<AircraftNode>, String>),
    ToggleAircraftSmartView,

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
    InstallProgress(f32),
    SmartSort,

    // Settings
    AddExclusion,
    ExclusionSelected(Option<PathBuf>),
    RemoveExclusion(PathBuf),

    // Heuristics
    OpenHeuristicsEditor,
    HeuristicsAction(text_editor::Action),
    SaveHeuristics,
    ImportHeuristics,
    ExportHeuristics,
    ResetHeuristics,
    HeuristicsImported(String),
    SetRegionFocus(Option<String>),

    // Simulation & Validation
    SimulationReportLoaded(
        Result<
            (
                Arc<Vec<SceneryPack>>,
                x_adox_core::scenery::validator::ValidationReport,
            ),
            String,
        >,
    ),
    ApplySort(Arc<Vec<SceneryPack>>),
    CancelSort,

    // Simulation Report interactions
    AutoFixIssue(String),        // Fixes all issues of a type
    IgnoreIssue(String, String), // Ignore specific issue (type, pack_name)
    ToggleIssueGroup(String),    // Toggle visibility of a group

    // Issues
    LogIssuesLoaded(Result<Arc<Vec<x_adox_core::LogIssue>>, String>),
    CheckLogIssues,

    // Sticky Sort (Phase 3)
    OpenPriorityEditor(String),
    UpdatePriorityValue(String, u8),
    SetPriority(String, u8),
    RemovePriority(String),
    CancelPriorityEdit,

    // Interactive Sorting (Phase 4)
    MovePack(String, MoveDirection),
    ClearAllPins,

    // Aircraft Override
    SetAircraftCategory(String, String), // Name, Category

    // Smart View Toggles
    ToggleSmartFolder(String),

    // Icon Customization
    BrowseForIcon(std::path::PathBuf), // Trigger file picker for this aircraft path
    IconSelected(std::path::PathBuf, std::path::PathBuf), // (Aircraft Path, Icon Path)

    // Profiles (Phase 2)
    ProfilesLoaded(Result<ProfileCollection, String>),
    SwitchProfile(String),
    SaveCurrentProfile(String),
    DeleteProfile(String),
    NewProfileNameChanged(String),
    OpenProfileDialog,
    CloseProfileDialog,
    OpenRenameDialog,
    RenameProfile(String, String), // (OldName, NewName)
    RenameProfileNameChanged(String),

    // Phase 3: Tags & Validation
    UpdateTagInput(String),
    AddTag,
    RemoveTag(String, String), // (PackName, Tag)
    TagOperationComplete,

    // Launch X-Plane
    LaunchXPlane,
    LaunchArgsChanged(String),
    // Utilities
    LogbookLoaded(Result<Vec<x_adox_core::logbook::LogbookEntry>, String>),
    SelectFlight(Option<usize>),
    AirportsLoaded(
        Result<Arc<std::collections::HashMap<String, x_adox_core::apt_dat::Airport>>, String>,
    ),
    ToggleLogbook,
}

#[derive(Debug, Clone, Copy, Default)]
struct LoadingState {
    is_loading: bool,
    scenery: bool,
    aircraft: bool,
    aircraft_tree: bool,
    plugins: bool,
    csls: bool,
    log_issues: bool,
    airports: bool,
    logbook: bool,
}

impl LoadingState {
    fn is_fully_loaded(&self) -> bool {
        self.scenery
            && self.aircraft
            && self.aircraft_tree
            && self.plugins
            && self.csls
            && self.log_issues
            && self.airports
            && self.logbook
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveDirection {
    Up,
    Down,
}

struct App {
    active_tab: Tab,
    packs: Arc<Vec<SceneryPack>>,
    aircraft: Arc<Vec<DiscoveredAddon>>,
    aircraft_tree: Option<Arc<AircraftNode>>,
    plugins: Arc<Vec<DiscoveredAddon>>,
    csls: Arc<Vec<DiscoveredAddon>>,
    status: String,
    xplane_root: Option<PathBuf>,
    available_xplane_roots: Vec<PathBuf>,
    selected_scenery: Option<String>,
    selected_aircraft: Option<PathBuf>,
    selected_aircraft_name: Option<String>,
    selected_aircraft_icon: Option<image::Handle>,
    selected_aircraft_tags: Vec<String>,
    selected_plugin: Option<PathBuf>,
    selected_csl: Option<PathBuf>,
    show_delete_confirm: bool,
    show_csl_tab: bool,
    use_smart_view: bool,
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
    install_progress: Option<f32>,
    // Heuristics
    heuristics_model: BitNetModel,
    heuristics_json: text_editor::Content,
    heuristics_error: Option<String>,

    // Issues
    log_issues: Arc<Vec<x_adox_core::LogIssue>>,
    icon_warning: svg::Handle,
    icon_pin: svg::Handle,
    icon_pin_outline: svg::Handle,
    icon_settings: svg::Handle,
    icon_utilities: svg::Handle,

    // Utilities State
    logbook: Vec<x_adox_core::logbook::LogbookEntry>,
    selected_flight: Option<usize>,
    airports: Arc<std::collections::HashMap<String, x_adox_core::apt_dat::Airport>>,
    logbook_expanded: bool,

    // Pro Mode
    validation_report: Option<x_adox_core::scenery::validator::ValidationReport>,
    simulated_packs: Option<Arc<Vec<SceneryPack>>>,
    region_focus: Option<String>,

    // UI State for polish
    ignored_issues: std::collections::HashSet<(String, String)>, // (type, pack_name)
    expanded_issue_groups: std::collections::HashSet<String>,    // type
    loading_state: LoadingState,

    // Sticky Sort Editing
    editing_priority: Option<(String, u8)>,
    // UI Helpers
    is_picking_exclusion: bool,
    // Phase 4 Icons
    icon_arrow_up: svg::Handle,
    icon_arrow_down: svg::Handle,
    icon_edit: svg::Handle,
    icon_trash: svg::Handle,

    // Fallback Icons
    fallback_airliner: image::Handle,
    fallback_ga: image::Handle,
    fallback_military: image::Handle,
    fallback_helicopter: image::Handle,

    // Smart View State
    smart_view_expanded: std::collections::BTreeSet<String>,

    // Icon Overrides
    icon_overrides: std::collections::BTreeMap<PathBuf, PathBuf>,

    // Scan Settings
    scan_exclusions: Vec<PathBuf>,
    scan_inclusions: Vec<PathBuf>,

    // Profiles (Phase 2)
    profile_manager: Option<ProfileManager>,
    profiles: ProfileCollection,
    new_profile_name: String,
    show_profile_dialog: bool,
    show_rename_dialog: bool,
    rename_profile_name: String,

    // Phase 3
    new_tag_input: String,

    // Launch X-Plane
    launch_args: String,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        // Discover all X-Plane installations
        let available_roots = XPlaneManager::find_all_xplane_roots();

        // Try to load persisted selection, fallback to first available or try_find_root
        let saved_root = Self::load_app_config();
        let root = if let Some(ref saved) = saved_root {
            if available_roots.contains(saved) || saved.exists() {
                Some(saved.clone())
            } else {
                available_roots.first().cloned()
            }
        } else {
            available_roots
                .first()
                .cloned()
                .or_else(XPlaneManager::try_find_root)
        };

        let mut app = Self {
            active_tab: Tab::Scenery,
            use_smart_view: false,
            packs: Arc::new(Vec::new()),
            aircraft: Arc::new(Vec::new()),
            aircraft_tree: None,
            plugins: Arc::new(Vec::new()),
            csls: Arc::new(Vec::new()),
            status: "Loading...".to_string(),
            xplane_root: root.clone(),
            available_xplane_roots: available_roots,
            selected_scenery: None,
            selected_aircraft: None,
            selected_aircraft_name: None,
            selected_aircraft_icon: None,
            selected_aircraft_tags: Vec::new(),
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
            install_progress: None,
            heuristics_model: BitNetModel::new().unwrap_or_default(),
            heuristics_json: text_editor::Content::new(),
            heuristics_error: None,
            log_issues: Arc::new(Vec::new()),
            icon_warning: svg::Handle::from_memory(
                include_bytes!("../assets/icons/warning.svg").to_vec(),
            ),
            icon_pin: svg::Handle::from_memory(include_bytes!("../assets/icons/pin.svg").to_vec()),
            icon_pin_outline: svg::Handle::from_memory(
                include_bytes!("../assets/icons/pin_outline.svg").to_vec(),
            ),
            icon_settings: svg::Handle::from_memory(
                include_bytes!("../assets/icons/settings.svg").to_vec(),
            ),
            icon_utilities: svg::Handle::from_memory(
                include_bytes!("../assets/icons/utilities.svg").to_vec(),
            ),
            simulated_packs: None,
            region_focus: None,
            ignored_issues: std::collections::HashSet::new(),
            expanded_issue_groups: std::collections::HashSet::new(),
            editing_priority: None,
            loading_state: LoadingState::default(),
            icon_arrow_up: svg::Handle::from_memory(
                include_bytes!("../assets/icons/arrow_up.svg").to_vec(),
            ),
            icon_arrow_down: svg::Handle::from_memory(
                include_bytes!("../assets/icons/arrow_down.svg").to_vec(),
            ),
            icon_edit: svg::Handle::from_memory(
                include_bytes!("../assets/icons/edit.svg").to_vec(),
            ),
            icon_trash: svg::Handle::from_memory(
                include_bytes!("../assets/icons/trash.svg").to_vec(),
            ),
            is_picking_exclusion: false,
            fallback_airliner: image::Handle::from_bytes(
                include_bytes!("../assets/fallback_airliner.png").to_vec(),
            ),
            fallback_ga: image::Handle::from_bytes(
                include_bytes!("../assets/fallback_ga.png").to_vec(),
            ),
            fallback_military: image::Handle::from_bytes(
                include_bytes!("../assets/fallback_military.png").to_vec(),
            ),
            fallback_helicopter: image::Handle::from_bytes(
                include_bytes!("../assets/fallback_helicopter.png").to_vec(),
            ),
            smart_view_expanded: std::collections::BTreeSet::new(),
            icon_overrides: std::collections::BTreeMap::new(),
            scan_exclusions: Vec::new(),
            scan_inclusions: Vec::new(),
            profile_manager: root.as_ref().map(|r| ProfileManager::new(r)),
            profiles: ProfileCollection::default(),
            new_profile_name: String::new(),
            show_profile_dialog: false,
            show_rename_dialog: false,
            rename_profile_name: String::new(),

            // Phase 3
            new_tag_input: String::new(),
            validation_report: None,

            // Utilities
            logbook: Vec::new(),
            selected_flight: None,
            airports: Arc::new(std::collections::HashMap::new()),
            logbook_expanded: false,

            // Launch X-Plane
            launch_args: String::new(),
        };

        if let Some(pm) = &app.profile_manager {
            if let Ok(collection) = pm.load() {
                app.profiles = collection;
            }
        }

        app.load_icon_overrides();
        app.load_scan_config();

        let tasks = if let Some(r) = root {
            app.loading_state.is_loading = true;

            let r1 = r.clone();
            let r2 = r.clone();
            let r3 = r.clone();
            let r4 = r.clone();
            let r5 = r.clone();
            let r6 = r.clone();
            let r7 = r.clone();
            let r8 = r.clone();

            let exclusions1 = app.scan_exclusions.clone();
            let exclusions2 = app.scan_exclusions.clone();

            Task::batch(vec![
                Task::perform(async move { load_packs(Some(r1)) }, Message::SceneryLoaded),
                Task::perform(
                    async move { load_aircraft(Some(r2), exclusions1) },
                    Message::AircraftLoaded,
                ),
                Task::perform(
                    async move { load_aircraft_tree(Some(r3), exclusions2) },
                    Message::AircraftTreeLoaded,
                ),
                Task::perform(
                    async move { load_plugins(Some(r4)) },
                    Message::PluginsLoaded,
                ),
                Task::perform(async move { load_csls(Some(r5)) }, Message::CSLsLoaded),
                Task::perform(
                    async move { load_log_issues(Some(r6)) },
                    Message::LogIssuesLoaded,
                ),
                Task::perform(load_airports_data(Some(r7)), Message::AirportsLoaded),
                Task::perform(load_logbook_data(Some(r8)), Message::LogbookLoaded),
            ])
        } else {
            Task::none()
        };

        (app, tasks)
    }

    fn check_loading_complete(&mut self) {
        if self.loading_state.is_loading && self.loading_state.is_fully_loaded() {
            self.loading_state.is_loading = false;
            self.status = "X-Plane Ready".to_string();
        }
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
            Message::InstallProgress(p) => {
                self.install_progress = Some(p);
                Task::none()
            }
            Message::SwitchTab(tab) => {
                self.active_tab = tab;
                // Update status to reflect current tab
                self.status = match tab {
                    Tab::Scenery => format!("{} scenery packs", self.packs.len()),
                    Tab::Aircraft => format!("{} aircraft", self.aircraft.len()),
                    Tab::Plugins => format!("{} plugins", self.plugins.len()),
                    Tab::CSLs => format!("{} CSL packages", self.csls.len()),
                    Tab::Heuristics => "Sorting Heuristics Editor".to_string(),
                    Tab::Issues => "Known Issues & Log Analysis".to_string(),
                    Tab::Utilities => "Utilities & Logbook Viewer".to_string(),
                    Tab::Settings => "Settings & Configuration".to_string(),
                };

                if tab == Tab::Utilities {
                    let root = self.xplane_root.clone();
                    return Task::perform(load_logbook_data(root), Message::LogbookLoaded);
                }

                Task::none()
            }
            Message::LogIssuesLoaded(result) => {
                self.loading_state.log_issues = true;
                match result {
                    Ok(issues) => {
                        self.log_issues = issues;
                        if !self.log_issues.is_empty() && !self.loading_state.is_loading {
                            self.status =
                                format!("Found {} issues in Log.txt", self.log_issues.len());
                        }
                    }
                    Err(e) => self.status = format!("Log analysis error: {}", e),
                }
                self.check_loading_complete();
                Task::none()
            }
            Message::CheckLogIssues => {
                let root = self.xplane_root.clone();
                Task::perform(
                    async move { load_log_issues(root) },
                    Message::LogIssuesLoaded,
                )
            }
            Message::SceneryLoaded(result) => {
                self.loading_state.scenery = true;
                match result {
                    Ok(packs) => {
                        self.packs = packs;
                        if !self.loading_state.is_loading {
                            self.status = format!("{} scenery packs", self.packs.len());

                            // Pipeline: Trigger next scan (only if not doing global reload)
                            let root = self.xplane_root.clone();
                            let exclusions = self.scan_exclusions.clone();
                            return Task::perform(
                                async move { load_aircraft(root, exclusions) },
                                Message::AircraftLoaded,
                            );
                        }
                    }
                    Err(e) => {
                        self.status = format!("Scenery load error: {}", e);
                    }
                }
                self.check_loading_complete();
                Task::none()
            }
            Message::AircraftLoaded(result) => {
                self.loading_state.aircraft = true;
                match result {
                    Ok(aircraft) => {
                        self.aircraft = aircraft;
                        if !self.loading_state.is_loading {
                            if self.active_tab == Tab::Aircraft {
                                self.status = format!("{} aircraft", self.aircraft.len());
                            }

                            // Pipeline: Trigger next scan
                            let root = self.xplane_root.clone();
                            let exclusions = self.scan_exclusions.clone();
                            return Task::perform(
                                async move { load_aircraft_tree(root, exclusions) },
                                Message::AircraftTreeLoaded,
                            );
                        }
                    }
                    Err(e) => {
                        if !self.loading_state.is_loading && self.active_tab == Tab::Aircraft {
                            self.status = format!("Aircraft error: {}", e);
                        }
                    }
                }
                self.check_loading_complete();
                Task::none()
            }
            Message::AircraftTreeLoaded(result) => {
                self.loading_state.aircraft_tree = true;
                match result {
                    Ok(tree) => {
                        self.aircraft_tree = Some(tree);
                        if !self.loading_state.is_loading {
                            if self.active_tab == Tab::Aircraft {
                                self.status = "Aircraft tree loaded".to_string();
                            }

                            // Pipeline: Trigger next scan
                            let root = self.xplane_root.clone();
                            return Task::perform(
                                async move { load_plugins(root) },
                                Message::PluginsLoaded,
                            );
                        }
                    }
                    Err(e) => {
                        if !self.loading_state.is_loading && self.active_tab == Tab::Aircraft {
                            self.status = format!("Aircraft tree error: {}", e);
                        }
                    }
                }
                self.check_loading_complete();
                Task::none()
            }
            Message::LogbookLoaded(result) => {
                self.loading_state.logbook = true;
                match result {
                    Ok(entries) => {
                        self.logbook = entries;
                        if !self.loading_state.is_loading {
                            self.status = format!("Loaded {} logbook entries", self.logbook.len());
                        }
                    }
                    Err(e) => self.status = format!("Logbook error: {}", e),
                }
                self.check_loading_complete();
                Task::none()
            }
            Message::AirportsLoaded(result) => {
                self.loading_state.airports = true;
                match result {
                    Ok(airports) => {
                        self.airports = airports;
                        if !self.loading_state.is_loading {
                            self.status = format!(
                                "Airport database loaded: {} airports",
                                self.airports.len()
                            );
                        }
                    }
                    Err(e) => self.status = format!("Airport database error: {}", e),
                }
                self.check_loading_complete();
                Task::none()
            }
            Message::SelectFlight(index) => {
                self.selected_flight = index;
                Task::none()
            }
            Message::ToggleLogbook => {
                self.logbook_expanded = !self.logbook_expanded;
                Task::none()
            }
            Message::PluginsLoaded(result) => {
                self.loading_state.plugins = true;
                match result {
                    Ok(plugins) => {
                        self.plugins = plugins;
                        if !self.loading_state.is_loading {
                            if self.active_tab == Tab::Plugins {
                                self.status = format!("{} plugins", self.plugins.len());
                            }

                            // Pipeline: Trigger next scan
                            let root = self.xplane_root.clone();
                            return Task::perform(
                                async move { load_csls(root) },
                                Message::CSLsLoaded,
                            );
                        }
                    }
                    Err(e) => {
                        if !self.loading_state.is_loading && self.active_tab == Tab::Plugins {
                            self.status = format!("Plugins error: {}", e);
                        }
                    }
                }
                self.check_loading_complete();
                Task::none()
            }
            Message::CSLsLoaded(result) => {
                self.loading_state.csls = true;
                match result {
                    Ok(csls) => {
                        self.csls = csls;
                        self.show_csl_tab = !self.csls.is_empty();

                        if !self.loading_state.is_loading {
                            if self.active_tab == Tab::CSLs {
                                self.status = format!("{} CSL packages", self.csls.len());
                            }

                            // Pipeline: Trigger next scan (Final: Log issues)
                            let root = self.xplane_root.clone();
                            return Task::perform(
                                async move { load_log_issues(root) },
                                Message::LogIssuesLoaded,
                            );
                        }
                    }
                    Err(e) => {
                        if !self.loading_state.is_loading && self.active_tab == Tab::CSLs {
                            self.status = format!("CSL error: {}", e);
                        }
                    }
                }
                self.check_loading_complete();
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
            Message::SwitchProfile(name) => {
                if let Some(profile) = self
                    .profiles
                    .profiles
                    .iter()
                    .find(|p| p.name == name)
                    .cloned()
                {
                    self.profiles.active_profile = Some(name.clone());
                    self.launch_args = profile.launch_args.clone(); // Load launch args from profile
                    let pm = self.profile_manager.clone();
                    let collection = self.profiles.clone();
                    let root = self.xplane_root.clone();

                    // Save active profile choice
                    if let Some(pm) = &pm {
                        let _ = pm.save(&collection);
                    }

                    self.status = format!("Switching to profile {}...", name);
                    Task::perform(
                        async move { apply_profile_task(root, profile).await },
                        |result: Result<(), String>| match result {
                            Ok(_) => Message::Refresh,
                            Err(e) => Message::ProfilesLoaded(Err(e)),
                        },
                    )
                } else {
                    Task::none()
                }
            }
            Message::SaveCurrentProfile(name) => {
                let pm = self.profile_manager.clone();
                let mut collection = self.profiles.clone();

                // Create profile from current state
                let scenery_states = self
                    .packs
                    .iter()
                    .map(|p| (p.name.clone(), p.status == SceneryPackType::Active))
                    .collect();
                let aircraft_states = self
                    .aircraft
                    .iter()
                    .map(|p| (p.path.to_string_lossy().to_string(), p.is_enabled))
                    .collect();
                let plugin_states = self
                    .plugins
                    .iter()
                    .map(|p| (p.path.to_string_lossy().to_string(), p.is_enabled))
                    .collect();

                let new_profile = Profile {
                    name: name.clone(),
                    scenery_states,
                    aircraft_states,
                    plugin_states,
                    launch_args: self.launch_args.clone(),
                };

                // Add or update
                if let Some(idx) = collection.profiles.iter().position(|p| p.name == name) {
                    collection.profiles[idx] = new_profile;
                } else {
                    collection.profiles.push(new_profile);
                }
                collection.active_profile = Some(name.clone());

                self.profiles = collection.clone();
                self.show_profile_dialog = false;
                self.new_profile_name = String::new();

                self.status = format!("Profile {} saved", name);
                Task::perform(
                    async move {
                        if let Some(pm) = pm {
                            pm.save(&collection).map_err(|e| e.to_string())?;
                        }
                        Ok::<(), String>(())
                    },
                    |result: Result<(), String>| match result {
                        Ok(_) => Message::Refresh,
                        Err(e) => Message::ProfilesLoaded(Err(e)),
                    },
                )
            }
            Message::DeleteProfile(name) => {
                self.profiles.profiles.retain(|p| p.name != name);
                if self.profiles.active_profile.as_ref() == Some(&name) {
                    self.profiles.active_profile = None;
                }
                let pm = self.profile_manager.clone();
                let collection = self.profiles.clone();

                self.status = format!("Deleted profile {}", name);
                Task::perform(
                    async move {
                        if let Some(pm) = pm {
                            pm.save(&collection).map_err(|e| e.to_string())?;
                        }
                        Ok::<(), String>(())
                    },
                    |_: Result<(), String>| Message::Refresh,
                )
            }
            Message::ProfilesLoaded(result) => {
                match result {
                    Ok(collection) => {
                        self.profiles = collection;
                        self.status = "Profiles loaded".to_string();
                    }
                    Err(e) => self.status = format!("Profiles error: {}", e),
                }
                Task::none()
            }
            Message::NewProfileNameChanged(name) => {
                self.new_profile_name = name;
                Task::none()
            }
            Message::OpenProfileDialog => {
                self.show_profile_dialog = true;
                Task::none()
            }
            Message::OpenRenameDialog => {
                if let Some(active) = &self.profiles.active_profile {
                    self.rename_profile_name = active.clone();
                    self.show_rename_dialog = true;
                }
                Task::none()
            }
            Message::RenameProfile(old_name, new_name) => {
                if new_name.is_empty() || old_name == new_name {
                    self.show_rename_dialog = false;
                    return Task::none();
                }

                if let Some(profile) = self
                    .profiles
                    .profiles
                    .iter_mut()
                    .find(|p| p.name == old_name)
                {
                    profile.name = new_name.clone();
                }

                if self.profiles.active_profile.as_ref() == Some(&old_name) {
                    self.profiles.active_profile = Some(new_name.clone());
                }

                let pm = self.profile_manager.clone();
                let collection = self.profiles.clone();

                self.show_rename_dialog = false;
                self.rename_profile_name = String::new();
                self.status = format!("Profile renamed to {}", new_name);

                Task::perform(
                    async move {
                        if let Some(pm) = pm {
                            pm.save(&collection).map_err(|e| e.to_string())?;
                        }
                        Ok::<(), String>(())
                    },
                    |result: Result<(), String>| match result {
                        Ok(_) => Message::Refresh,
                        Err(e) => Message::ProfilesLoaded(Err(e)),
                    },
                )
            }
            Message::RenameProfileNameChanged(name) => {
                self.rename_profile_name = name;
                Task::none()
            }
            Message::LaunchXPlane => {
                if let Some(ref root) = self.xplane_root {
                    match XPlaneManager::new(root) {
                        Ok(manager) => {
                            if let Some(exe) = manager.get_executable_path() {
                                let args_vec: Vec<&str> = if self.launch_args.is_empty() {
                                    vec![]
                                } else {
                                    self.launch_args.split_whitespace().collect()
                                };

                                match std::process::Command::new(&exe)
                                    .args(&args_vec)
                                    .current_dir(root)
                                    .spawn()
                                {
                                    Ok(_) => {
                                        self.status = "X-Plane launched!".to_string();
                                    }
                                    Err(e) => {
                                        self.status = format!("Failed to launch X-Plane: {}", e);
                                    }
                                }
                            } else {
                                self.status = "X-Plane executable not found".to_string();
                            }
                        }
                        Err(e) => {
                            self.status = format!("Invalid X-Plane root: {}", e);
                        }
                    }
                } else {
                    self.status = "No X-Plane installation selected".to_string();
                }
                Task::none()
            }
            Message::LaunchArgsChanged(args) => {
                self.launch_args = args.clone();

                // Update current profile's launch_args if there's an active one
                if let Some(ref active_name) = self.profiles.active_profile.clone() {
                    if let Some(profile) = self
                        .profiles
                        .profiles
                        .iter_mut()
                        .find(|p| p.name == *active_name)
                    {
                        profile.launch_args = args;
                    }

                    // Save profiles
                    if let Some(ref pm) = self.profile_manager {
                        let _ = pm.save(&self.profiles);
                    }
                }
                Task::none()
            }
            Message::TagOperationComplete => Task::none(),
            Message::UpdateTagInput(txt) => {
                self.new_tag_input = txt;
                Task::none()
            }
            Message::AddTag => {
                let tag = self.new_tag_input.trim().to_string();
                if !tag.is_empty() {
                    if let Some(pack_name) = self.selected_scenery.clone() {
                        let packs = Arc::make_mut(&mut self.packs);
                        if let Some(pack) = packs.iter_mut().find(|p| p.name == pack_name) {
                            if !pack.tags.contains(&tag) {
                                pack.tags.push(tag.clone());

                                let root = self.xplane_root.clone();
                                let p_name = pack_name.clone();
                                let t_val = tag.clone();
                                self.new_tag_input.clear();

                                return Task::perform(
                                    async move {
                                        if let Some(r) = root {
                                            let mgr = x_adox_core::groups::GroupManager::new(&r);
                                            if let Ok(mut col) = mgr.load() {
                                                let list = col.pack_tags.entry(p_name).or_default();
                                                if !list.contains(&t_val) {
                                                    list.push(t_val);
                                                    let _ = mgr.save(&col);
                                                }
                                            }
                                        }
                                    },
                                    |_| Message::TagOperationComplete,
                                );
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::RemoveTag(pack_name, tag) => {
                let packs = Arc::make_mut(&mut self.packs);
                if let Some(pack) = packs.iter_mut().find(|p| p.name == pack_name) {
                    if let Some(pos) = pack.tags.iter().position(|t| t == &tag) {
                        pack.tags.remove(pos);

                        let root = self.xplane_root.clone();
                        let p_name = pack_name.clone();
                        let t_val = tag.clone();

                        return Task::perform(
                            async move {
                                if let Some(r) = root {
                                    let mgr = x_adox_core::groups::GroupManager::new(&r);
                                    if let Ok(mut col) = mgr.load() {
                                        if let Some(list) = col.pack_tags.get_mut(&p_name) {
                                            if let Some(idx) = list.iter().position(|x| x == &t_val)
                                            {
                                                list.remove(idx);
                                                let _ = mgr.save(&col);
                                            }
                                        }
                                    }
                                }
                            },
                            |_| Message::TagOperationComplete,
                        );
                    }
                }
                Task::none()
            }
            Message::CloseProfileDialog => {
                self.show_profile_dialog = false;
                self.show_rename_dialog = false;
                Task::none()
            }
            Message::ToggleAircraft(path, enable) => {
                let root = self.xplane_root.clone();
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                self.status = format!(
                    "{} Aircraft {}...",
                    if enable { "Enabling" } else { "Disabling" },
                    name
                );
                Task::perform(
                    async move { toggle_aircraft(root, path, enable) },
                    Message::AircraftToggled,
                )
            }
            Message::AircraftToggled(result) => match result {
                Ok(_) => {
                    self.status = "Aircraft toggled!".to_string();
                    Task::done(Message::Refresh)
                }
                Err(e) => {
                    self.status = format!("Error toggling aircraft: {}", e);
                    Task::none()
                }
            },
            Message::ToggleAircraftSmartView => {
                self.use_smart_view = !self.use_smart_view;
                Task::none()
            }
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
            Message::SmartSort => {
                let root = self.xplane_root.clone();
                let context = x_adox_bitnet::PredictContext {
                    region_focus: self.region_focus.clone(),
                };
                let model = self.heuristics_model.clone();
                self.status = "Simulating sort...".to_string();
                Task::perform(
                    async move { simulate_sort_task(root, model, context) },
                    Message::SimulationReportLoaded,
                )
            }
            Message::SimulationReportLoaded(result) => {
                match result {
                    Ok((packs, report)) => {
                        self.simulated_packs = Some(packs);
                        self.validation_report = Some(report);
                        self.status = "Simulation complete. Review warnings if any.".to_string();
                    }
                    Err(e) => self.status = format!("Simulation error: {}", e),
                }
                Task::none()
            }
            Message::ApplySort(packs) => {
                self.packs = packs;
                let packs_to_save = self.packs.clone();
                let root = self.xplane_root.clone();
                self.simulated_packs = None;
                self.validation_report = None;
                self.status = "Applying changes...".to_string();
                Task::perform(
                    async move { save_packs_task(root, packs_to_save) },
                    Message::PackToggled,
                )
            }
            Message::CancelSort => {
                self.simulated_packs = None;
                self.validation_report = None;
                self.status = "Sort cancelled.".to_string();
                Task::none()
            }
            Message::SetRegionFocus(focus) => {
                self.region_focus = focus;
                Task::none()
            }
            Message::AutoFixIssue(issue_type) => {
                if let Some(ref mut packs_arc) = self.simulated_packs {
                    let packs = Arc::make_mut(packs_arc);
                    match issue_type.as_str() {
                        "simheaven_below_global" => {
                            if let Some(_ga_idx) = packs
                                .iter()
                                .position(|p| p.category == SceneryCategory::GlobalAirport)
                            {
                                let mut to_move = Vec::new();
                                // Collect all simheaven/x-world packs ABOVE GA
                                let mut i = 0;
                                // Need to recalculate limit as we remove items
                                loop {
                                    let ga_current = packs
                                        .iter()
                                        .position(|p| p.category == SceneryCategory::GlobalAirport);
                                    if let Some(ga_idx_now) = ga_current {
                                        if i >= ga_idx_now {
                                            break;
                                        }
                                        let name = packs[i].name.to_lowercase();
                                        if name.contains("simheaven") || name.contains("x-world") {
                                            to_move.push(packs.remove(i));
                                        } else {
                                            i += 1;
                                        }
                                    } else {
                                        break;
                                    }
                                }

                                // Move to just BELOW ga_idx
                                if let Some(new_ga_idx) = packs
                                    .iter()
                                    .position(|p| p.category == SceneryCategory::GlobalAirport)
                                {
                                    let insert_pos = new_ga_idx + 1;
                                    for pack in to_move.into_iter().rev() {
                                        if insert_pos <= packs.len() {
                                            packs.insert(insert_pos, pack);
                                        } else {
                                            packs.push(pack);
                                        }
                                    }
                                }

                                // PERSISTENCE: Update the BitNet rules so it stays fixed!
                                let mut rules_updated = false;
                                for rule in
                                    &mut Arc::make_mut(&mut self.heuristics_model.config).rules
                                {
                                    if rule.name.contains("SimHeaven") {
                                        rule.score = 30; // Matches lib.rs refined score
                                        rules_updated = true;
                                    }
                                    if rule.name.contains("Global Airports") {
                                        rule.score = 20; // Matches lib.rs refined score
                                        rules_updated = true;
                                    }
                                }

                                if rules_updated {
                                    let _ = self.heuristics_model.save();
                                }
                            }
                        }
                        "mesh_above_overlay" => {
                            let mut meshes = Vec::new();
                            let mut i = 0;
                            while i < packs.len() {
                                if packs[i].category == SceneryCategory::Mesh
                                    || packs[i].category == SceneryCategory::Ortho
                                {
                                    meshes.push(packs.remove(i));
                                } else {
                                    i += 1;
                                }
                            }
                            packs.extend(meshes);

                            // PERSISTENCE: Update the BitNet rules for Mesh ordering
                            let mut rules_updated = false;
                            for rule in &mut Arc::make_mut(&mut self.heuristics_model.config).rules
                            {
                                if rule.name.contains("Mesh") {
                                    rule.score = 60;
                                    rules_updated = true;
                                }
                                if rule.name.contains("Ortho") || rule.name.contains("Overlay") {
                                    rule.score = 50; // Ortho baseline is 50, AutoOrtho Overlays is 48
                                    rules_updated = true;
                                }
                            }

                            if rules_updated {
                                let _ = self.heuristics_model.save();
                            }
                        }
                        "shadowed_mesh" => {
                            if let Some(report) = &self.validation_report {
                                let mut packs_to_disable = std::collections::HashSet::new();
                                for issue in &report.issues {
                                    if issue.issue_type == "shadowed_mesh"
                                        && !self.ignored_issues.contains(&(
                                            issue.issue_type.clone(),
                                            issue.pack_name.clone(),
                                        ))
                                    {
                                        packs_to_disable.insert(issue.pack_name.clone());
                                    }
                                }
                                for pack in packs.iter_mut() {
                                    if packs_to_disable.contains(&pack.name) {
                                        pack.status =
                                            x_adox_core::scenery::SceneryPackType::Disabled;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    // Re-validate
                    let report = x_adox_core::scenery::validator::SceneryValidator::validate(packs);
                    self.validation_report = Some(report);
                }
                Task::none()
            }
            Message::IgnoreIssue(issue_type, pack_name) => {
                self.ignored_issues.insert((issue_type, pack_name));
                Task::none()
            }
            Message::ToggleIssueGroup(issue_type) => {
                if self.expanded_issue_groups.contains(&issue_type) {
                    self.expanded_issue_groups.remove(&issue_type);
                } else {
                    self.expanded_issue_groups.insert(issue_type);
                }
                Task::none()
            }
            Message::OpenPriorityEditor(pack_name) => {
                let current_score = self.heuristics_model.predict(
                    &pack_name,
                    std::path::Path::new(""), // Not used in predict if it's an override
                    &x_adox_bitnet::PredictContext {
                        region_focus: self.region_focus.clone(),
                    },
                );
                self.editing_priority = Some((pack_name, current_score));
                Task::none()
            }
            Message::UpdatePriorityValue(pack_name, score) => {
                self.editing_priority = Some((pack_name, score));
                Task::none()
            }
            Message::SetPriority(pack_name, score) => {
                Arc::make_mut(&mut self.heuristics_model.config)
                    .overrides
                    .insert(pack_name, score);
                self.heuristics_model.refresh_regex_set();
                let _ = self.heuristics_model.save();
                self.editing_priority = None;
                Task::none()
            }
            Message::RemovePriority(pack_name) => {
                Arc::make_mut(&mut self.heuristics_model.config)
                    .overrides
                    .remove(&pack_name);
                self.heuristics_model.refresh_regex_set();
                let _ = self.heuristics_model.save();
                self.editing_priority = None;
                Task::none()
            }
            Message::CancelPriorityEdit => {
                self.editing_priority = None;
                Task::none()
            }
            Message::MovePack(name, direction) => {
                if let Some(idx) = self.packs.iter().position(|p| p.name == name) {
                    let neighbor_idx = match direction {
                        MoveDirection::Up => {
                            if idx == 0 {
                                None
                            } else {
                                Some(idx - 1)
                            }
                        }
                        MoveDirection::Down => {
                            if idx + 1 >= self.packs.len() {
                                None
                            } else {
                                Some(idx + 1)
                            }
                        }
                    };

                    if let Some(n_idx) = neighbor_idx {
                        let neighbor_name = self.packs[n_idx].name.clone();

                        // Get neighbor's current score
                        let neighbor_score = self.heuristics_model.predict(
                            &neighbor_name,
                            std::path::Path::new(""),
                            &x_adox_bitnet::PredictContext {
                                region_focus: self.region_focus.clone(),
                            },
                        );

                        // Calculate new score for our target
                        let new_score = match direction {
                            MoveDirection::Up => neighbor_score.saturating_sub(1),
                            MoveDirection::Down => neighbor_score.saturating_add(1),
                        };

                        // Pin it!
                        Arc::make_mut(&mut self.heuristics_model.config)
                            .overrides
                            .insert(name.clone(), new_score);
                        self.heuristics_model.refresh_regex_set();
                        let _ = self.heuristics_model.save();

                        // Locally swap to provide instant feedback
                        Arc::make_mut(&mut self.packs).swap(idx, n_idx);
                        self.status = format!("Moved {} and pinned to score {}", name, new_score);
                    }
                }
                Task::none()
            }
            Message::ClearAllPins => {
                Arc::make_mut(&mut self.heuristics_model.config)
                    .overrides
                    .clear();
                self.heuristics_model.refresh_regex_set();
                let _ = self.heuristics_model.save();
                self.resort_scenery();
                self.status = "All manual reorder pins cleared".to_string();
                Task::none()
            }
            Message::SetAircraftCategory(name, category) => {
                // Determine tags based on selected category
                // For now, let's keep it simple: one tag if it's a known category,
                // or we could append it to manufacturer?
                // The implementation plan says: "it will replace the predicted tags."
                let tags = if category == "Airliner" {
                    vec!["Airliner".to_string(), "Jet".to_string()]
                } else if category == "General Aviation" {
                    vec!["General Aviation".to_string(), "Prop".to_string()]
                } else if category == "Military" {
                    vec!["Military".to_string(), "Jet".to_string()]
                } else if category == "Helicopter" {
                    vec!["Helicopter".to_string()]
                } else if category == "Glider" {
                    vec!["Glider".to_string()]
                } else if category == "Business Jet" {
                    vec![
                        "General Aviation".to_string(),
                        "Business Jet".to_string(),
                        "Jet".to_string(),
                    ]
                } else {
                    vec![category.clone()]
                };

                Arc::make_mut(&mut self.heuristics_model.config)
                    .aircraft_overrides
                    .insert(name, tags);
                self.heuristics_model.refresh_regex_set();
                let _ = self.heuristics_model.save();

                // Refresh to show changes
                Task::done(Message::Refresh)
            }
            Message::Refresh => {
                self.status = "Refreshing...".to_string();
                let root = self.xplane_root.clone();
                Task::perform(async move { load_packs(root) }, Message::SceneryLoaded)
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
                            // Add to available roots if not already present
                            if !self.available_xplane_roots.contains(&path) {
                                self.available_xplane_roots.push(path.clone());
                            }
                            self.xplane_root = Some(path);
                            self.save_app_config();
                            self.profile_manager =
                                self.xplane_root.as_ref().map(|r| ProfileManager::new(r));
                            self.status = "X-Plane folder set! Reloading...".to_string();

                            // Reset loading state
                            self.loading_state = LoadingState {
                                is_loading: true,
                                ..Default::default()
                            };

                            let root1 = self.xplane_root.clone();
                            let root2 = self.xplane_root.clone();
                            let root3 = self.xplane_root.clone();
                            let root4 = self.xplane_root.clone();
                            let root5 = self.xplane_root.clone();
                            let root6 = self.xplane_root.clone();
                            let root7 = self.xplane_root.clone();
                            let root8 = self.xplane_root.clone();

                            let ex1 = self.scan_exclusions.clone();
                            let ex2 = self.scan_exclusions.clone();

                            return Task::batch([
                                Task::perform(
                                    async move { load_packs(root1) },
                                    Message::SceneryLoaded,
                                ),
                                Task::perform(
                                    async move { load_aircraft(root2, ex1) },
                                    Message::AircraftLoaded,
                                ),
                                Task::perform(
                                    async move { load_aircraft_tree(root3, ex2) },
                                    Message::AircraftTreeLoaded,
                                ),
                                Task::perform(
                                    async move { load_plugins(root4) },
                                    Message::PluginsLoaded,
                                ),
                                Task::perform(async move { load_csls(root5) }, Message::CSLsLoaded),
                                Task::perform(
                                    async move { load_log_issues(root6) },
                                    Message::LogIssuesLoaded,
                                ),
                                Task::perform(load_airports_data(root7), Message::AirportsLoaded),
                                Task::perform(load_logbook_data(root8), Message::LogbookLoaded),
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
            Message::SelectXPlaneRoot(path) => {
                if self.xplane_root.as_ref() == Some(&path) {
                    return Task::none(); // Already selected
                }

                self.xplane_root = Some(path);
                self.save_app_config();
                self.profile_manager = self.xplane_root.as_ref().map(|r| ProfileManager::new(r));
                self.status = "Switching X-Plane installation...".to_string();

                // Reset loading state
                self.loading_state = LoadingState {
                    is_loading: true,
                    ..Default::default()
                };

                let root1 = self.xplane_root.clone();
                let root2 = self.xplane_root.clone();
                let root3 = self.xplane_root.clone();
                let root4 = self.xplane_root.clone();
                let root5 = self.xplane_root.clone();
                let root6 = self.xplane_root.clone();
                let root7 = self.xplane_root.clone();
                let root8 = self.xplane_root.clone();

                let ex1 = self.scan_exclusions.clone();
                let ex2 = self.scan_exclusions.clone();

                Task::batch([
                    Task::perform(async move { load_packs(root1) }, Message::SceneryLoaded),
                    Task::perform(
                        async move { load_aircraft(root2, ex1) },
                        Message::AircraftLoaded,
                    ),
                    Task::perform(
                        async move { load_aircraft_tree(root3, ex2) },
                        Message::AircraftTreeLoaded,
                    ),
                    Task::perform(async move { load_plugins(root4) }, Message::PluginsLoaded),
                    Task::perform(async move { load_csls(root5) }, Message::CSLsLoaded),
                    Task::perform(
                        async move { load_log_issues(root6) },
                        Message::LogIssuesLoaded,
                    ),
                    Task::perform(load_airports_data(root7), Message::AirportsLoaded),
                    Task::perform(load_logbook_data(root8), Message::LogbookLoaded),
                ])
            }
            Message::ToggleAircraftFolder(path) => {
                if let Some(ref mut tree) = self.aircraft_tree {
                    toggle_folder_at_path(Arc::make_mut(tree), &path);
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
                self.selected_aircraft_name =
                    path.file_name().map(|n| n.to_string_lossy().to_string());

                // Get tags
                if let Some(tree) = &self.aircraft_tree {
                    self.selected_aircraft_tags =
                        Self::find_tags_in_tree(tree, &path).unwrap_or_default();
                } else {
                    self.selected_aircraft_tags = Vec::new();
                }

                // Improved icon search
                let mut icon_handle = None;

                // 0. Check Overrides
                if let Some(override_path) = self.icon_overrides.get(&path) {
                    if let Ok(bytes) = std::fs::read(override_path) {
                        icon_handle = Some(image::Handle::from_bytes(bytes));
                    }
                }

                // 1. Check immediate directory and parent directory (for shared icons/CSLs)
                let mut search_dirs = Vec::new();
                if icon_handle.is_none() {
                    search_dirs.push(path.clone());
                    if let Some(parent) = path.parent() {
                        search_dirs.push(parent.to_path_buf());
                    }
                }

                for dir in search_dirs {
                    if let Ok(entries) = std::fs::read_dir(&dir) {
                        let entries_vec: Vec<_> = entries.flatten().collect();

                        // Try to find .acf to get stem name
                        let acf_stem = entries_vec
                            .iter()
                            .find(|e| e.path().extension().map_or(false, |ext| ext == "acf"))
                            .and_then(|e| {
                                e.path()
                                    .file_stem()
                                    .map(|s| s.to_string_lossy().to_string())
                            });

                        // Candidates based on standard naming
                        let mut candidates = Vec::new();
                        if let Some(stem) = &acf_stem {
                            candidates.push(dir.join(format!("{}_icon11.png", stem)));
                            candidates.push(dir.join(format!("{}_icon.png", stem)));
                        }
                        candidates.push(dir.join("icon11.png"));
                        candidates.push(dir.join("icon.png"));

                        for p in candidates {
                            if p.exists() {
                                if let Ok(bytes) = std::fs::read(&p) {
                                    icon_handle = Some(image::Handle::from_bytes(bytes));
                                    break;
                                }
                            }
                        }

                        if icon_handle.is_some() {
                            break;
                        }

                        // Fallback: look for ANY png that might be an icon
                        for e in entries_vec {
                            let name = e.file_name().to_string_lossy().to_lowercase();
                            if name.contains("icon")
                                || name.contains("preview")
                                || name.contains("thumbnail")
                            {
                                if let Ok(bytes) = std::fs::read(e.path()) {
                                    icon_handle = Some(image::Handle::from_bytes(bytes));
                                    break;
                                }
                            }
                        }
                    }
                    if icon_handle.is_some() {
                        break;
                    }
                }

                // 2. High-quality category-based fallback if still no icon found
                if icon_handle.is_none() {
                    let tags = &self.selected_aircraft_tags;
                    let fallback = if tags
                        .iter()
                        .any(|t| t.contains("Airliner") || t.contains("Jet"))
                    {
                        self.fallback_airliner.clone()
                    } else if tags
                        .iter()
                        .any(|t| t.contains("Military") || t.contains("Combat"))
                    {
                        self.fallback_military.clone()
                    } else if tags.iter().any(|t| t.contains("Helicopter")) {
                        self.fallback_helicopter.clone()
                    } else {
                        self.fallback_ga.clone()
                    };
                    icon_handle = Some(fallback);
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
                    self.install_progress = Some(0.0);

                    return Task::run(
                        iced::stream::channel(
                            10,
                            move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                                let mut output_progress = output.clone();
                                let res = install_addon(root, zip_path, tab, None, move |p| {
                                    let _ = output_progress.try_send(Message::InstallProgress(p));
                                })
                                .await;
                                let _ = output.try_send(Message::InstallComplete(res));
                            },
                        ),
                        |msg| msg,
                    );
                } else {
                    self.status = "Install cancelled".to_string();
                    Task::none()
                }
            }
            Message::InstallAircraftDestPicked(zip_path, dest_opt) => {
                if let Some(dest_path) = dest_opt {
                    let root = self.xplane_root.clone();
                    self.status = format!("Installing to {}...", dest_path.display());
                    self.install_progress = Some(0.0);

                    return Task::run(
                        iced::stream::channel(
                            10,
                            move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                                let mut output_progress = output.clone();
                                let res = install_addon(
                                    root,
                                    zip_path,
                                    Tab::Aircraft,
                                    Some(dest_path),
                                    move |p| {
                                        let _ =
                                            output_progress.try_send(Message::InstallProgress(p));
                                    },
                                )
                                .await;
                                let _ = output.try_send(Message::InstallComplete(res));
                            },
                        ),
                        |msg| msg,
                    );
                } else {
                    self.status = "Install cancelled (no destination selected)".to_string();
                    Task::none()
                }
            }
            Message::InstallComplete(result) => {
                self.install_progress = None;
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
                let name_opt = match tab {
                    Tab::Scenery => self.selected_scenery.clone(),
                    Tab::Aircraft => self
                        .selected_aircraft
                        .as_ref()
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())),
                    Tab::Plugins => self
                        .selected_plugin
                        .as_ref()
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())),
                    Tab::CSLs => self
                        .selected_csl
                        .as_ref()
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())),
                    Tab::Heuristics | Tab::Issues | Tab::Settings | Tab::Utilities => None,
                };

                if let Some(name) = name_opt {
                    use native_dialog::{MessageDialog, MessageType};
                    let confirmed = MessageDialog::new()
                        .set_title("Confirm Deletion")
                        .set_text(&format!(
                            "Are you sure you want to permanently delete '{}'?",
                            name
                        ))
                        .set_type(MessageType::Warning)
                        .show_confirm()
                        .unwrap_or(false);

                    if confirmed {
                        return Task::done(Message::ConfirmDelete(tab, true));
                    }
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
                        Tab::Heuristics | Tab::Issues | Tab::Settings | Tab::Utilities => None,
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
            Message::OpenHeuristicsEditor => {
                let json = serde_json::to_string_pretty(self.heuristics_model.config.as_ref())
                    .unwrap_or_default();
                self.heuristics_json = text_editor::Content::with_text(&json);
                self.heuristics_error = None;
                self.active_tab = Tab::Heuristics;
                self.status = "Sorting Heuristics Editor".to_string();
                Task::none()
            }
            Message::HeuristicsAction(action) => {
                self.heuristics_json.perform(action);
                Task::none()
            }
            Message::SaveHeuristics => {
                let text = self.heuristics_json.text();
                match serde_json::from_str::<x_adox_bitnet::HeuristicsConfig>(&text) {
                    Ok(config) => {
                        self.heuristics_model.update_config(config);
                        if let Err(e) = self.heuristics_model.save() {
                            self.heuristics_error = Some(format!("Save failed: {}", e));
                        } else {
                            self.heuristics_error = None;
                            self.status = "Heuristics saved!".to_string();
                        }
                    }
                    Err(e) => {
                        self.heuristics_error = Some(format!("JSON Error: {}", e));
                    }
                }
                Task::none()
            }
            Message::ResetHeuristics => {
                if let Err(e) = self.heuristics_model.reset_defaults() {
                    self.heuristics_error = Some(format!("Reset failed: {}", e));
                } else {
                    self.heuristics_model.refresh_regex_set();
                    let json = serde_json::to_string_pretty(self.heuristics_model.config.as_ref())
                        .unwrap_or_default();
                    self.heuristics_json = text_editor::Content::with_text(&json);
                    self.heuristics_error = None;
                    self.status = "Heuristics reset to defaults".to_string();
                }
                Task::none()
            }
            Message::ImportHeuristics => Task::perform(
                async {
                    use native_dialog::FileDialog;
                    FileDialog::new()
                        .set_title("Import Heuristics JSON")
                        .add_filter("JSON", &["json"])
                        .show_open_single_file()
                        .ok()
                        .flatten()
                },
                |path_opt| {
                    if let Some(path) = path_opt {
                        if let Ok(text) = std::fs::read_to_string(path) {
                            return Message::HeuristicsImported(text);
                        }
                    }
                    Message::Refresh // No-op refresh
                },
            ),
            Message::HeuristicsImported(text) => {
                self.heuristics_json = text_editor::Content::with_text(&text);
                self.heuristics_error = None;
                self.status = "JSON imported. Click Save to apply.".to_string();
                Task::none()
            }
            Message::ExportHeuristics => {
                let text = self.heuristics_json.text();
                Task::perform(
                    async move {
                        use native_dialog::FileDialog;
                        FileDialog::new()
                            .set_title("Export Heuristics JSON")
                            .add_filter("JSON", &["json"])
                            .show_save_single_file()
                            .ok()
                            .flatten()
                    },
                    move |path_opt| {
                        if let Some(path) = path_opt {
                            let _ = std::fs::write(path, &text);
                        }
                        Message::Refresh
                    },
                )
            }
            Message::AddExclusion => {
                if self.is_picking_exclusion {
                    return Task::none();
                }
                self.is_picking_exclusion = true;
                Task::perform(
                    async move {
                        use native_dialog::FileDialog;
                        FileDialog::new()
                            .set_location("~")
                            .show_open_single_dir()
                            .ok()
                            .flatten()
                    },
                    Message::ExclusionSelected,
                )
            }
            Message::ExclusionSelected(Some(path)) => {
                self.is_picking_exclusion = false;
                let final_path = path.canonicalize().unwrap_or(path);
                if !self.scan_exclusions.contains(&final_path) {
                    self.scan_exclusions.push(final_path);
                    let _ = self.save_scan_config();
                    return Task::done(Message::Refresh);
                }
                Task::none()
            }
            Message::ExclusionSelected(None) => {
                self.is_picking_exclusion = false;
                Task::none()
            }
            Message::RemoveExclusion(path) => {
                if let Some(pos) = self.scan_exclusions.iter().position(|x| *x == path) {
                    self.scan_exclusions.remove(pos);
                    self.save_scan_config();
                    return Task::done(Message::Refresh);
                }
                Task::none()
            }
            Message::ToggleSmartFolder(id) => {
                if self.smart_view_expanded.contains(&id) {
                    self.smart_view_expanded.remove(&id);
                } else {
                    self.smart_view_expanded.insert(id);
                }
                Task::none()
            }
            Message::BrowseForIcon(path) => {
                let path_c = path.clone();
                Task::perform(
                    async move {
                        let result = tokio::task::spawn_blocking(move || {
                            native_dialog::FileDialog::new()
                                .set_location(&path_c)
                                .set_title("Select Custom Aircraft Icon")
                                .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                                .show_open_single_file()
                        })
                        .await;
                        match result {
                            Ok(Ok(Some(icon_path))) => Some((path, icon_path)),
                            _ => None,
                        }
                    },
                    |res| {
                        if let Some((path, icon)) = res {
                            Message::IconSelected(path, icon)
                        } else {
                            Message::Refresh // No-op
                        }
                    },
                )
            }
            Message::IconSelected(acf_path, icon_path) => {
                self.icon_overrides.insert(acf_path.clone(), icon_path);
                self.save_icon_overrides();

                // If the modified aircraft is currently selected, re-select it to refresh the icon
                if let Some(selected) = &self.selected_aircraft {
                    if selected == &acf_path {
                        return Task::done(Message::SelectAircraft(acf_path));
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

        let main_view = row![
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
        .height(Length::Fill);

        if self.loading_state.is_loading {
            self.view_loading_overlay()
        } else if let Some((pack_name, score)) = &self.editing_priority {
            container(self.view_priority_editor(pack_name, *score))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_| container::Style {
                    background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.85))),
                    ..Default::default()
                })
                .into()
        } else if let (Some(report), Some(packs)) = (&self.validation_report, &self.simulated_packs)
        {
            container(self.view_simulation_modal(report, packs))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_| container::Style {
                    background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.85))),
                    ..Default::default()
                })
                .into()
        } else {
            main_view.into()
        }
    }

    fn view_loading_overlay(&self) -> Element<'_, Message> {
        let title = text("Switching X-Plane Installation")
            .size(24)
            .color(style::palette::TEXT_PRIMARY);

        let subtitle = text("Synchronizing all data sources...")
            .size(16)
            .color(style::palette::TEXT_SECONDARY);

        let items = [
            ("Scenery Library", self.loading_state.scenery),
            ("Aircraft Addons", self.loading_state.aircraft),
            ("Aircraft Tree Structure", self.loading_state.aircraft_tree),
            ("Plugins", self.loading_state.plugins),
            ("CSL Packages", self.loading_state.csls),
            ("Log Issues Analysis", self.loading_state.log_issues),
            ("Airport Database", self.loading_state.airports),
            ("Pilot Logbook", self.loading_state.logbook),
        ];

        let mut progress = Column::new().spacing(12);
        for (label, done) in items {
            let status_indicator =
                container("")
                    .width(10)
                    .height(10)
                    .style(move |_| container::Style {
                        background: Some(Background::Color(if done {
                            style::palette::ACCENT_GREEN
                        } else {
                            Color::from_rgba(0.5, 0.5, 0.5, 0.2)
                        })),
                        border: Border {
                            color: if done {
                                style::palette::ACCENT_GREEN
                            } else {
                                style::palette::BORDER
                            },
                            width: 1.0,
                            radius: 5.0.into(),
                        },
                        ..Default::default()
                    });

            progress = progress.push(
                row![
                    status_indicator,
                    text(label.to_string()).color(if done {
                        style::palette::TEXT_PRIMARY
                    } else {
                        style::palette::TEXT_SECONDARY
                    }),
                ]
                .spacing(15)
                .align_y(iced::Alignment::Center),
            );
        }

        container(
            container(
                column![title, subtitle, progress]
                    .spacing(20)
                    .align_x(iced::Alignment::Center),
            )
            .style(style::container_modal)
            .padding(40)
            .width(Length::Shrink),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.85))),
            ..Default::default()
        })
        .into()
    }

    fn view_priority_editor<'a>(
        &'a self,
        pack_name: &'a str,
        current_score: u8,
    ) -> Element<'a, Message> {
        let title = text(format!("Set Custom Priority: {}", pack_name))
            .size(20)
            .color(style::palette::TEXT_PRIMARY);

        // Live slider updates are handled via UpdatePriorityValue

        container(
            column![
                title,
                text(format!(
                    "Priority Score: {} (Lower = Higher Priority)",
                    current_score
                ))
                .size(14)
                .color(style::palette::TEXT_SECONDARY),
                row![
                    text("0 (Top)").size(10),
                    slider(0..=100, current_score, move |s| {
                        Message::UpdatePriorityValue(pack_name.to_string(), s)
                    })
                    .width(Length::Fill),
                    text("100 (Bottom)").size(10),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center),
                row![
                    button(text("Reset Default").size(12))
                        .on_press(Message::RemovePriority(pack_name.to_string()))
                        .style(style::button_secondary)
                        .padding([8, 16]),
                    iced::widget::Space::with_width(Length::Fill),
                    button(text("Cancel").size(12))
                        .on_press(Message::CancelPriorityEdit)
                        .style(style::button_secondary)
                        .padding([8, 16]),
                    button(text("Apply").size(12))
                        .on_press(Message::SetPriority(pack_name.to_string(), current_score))
                        .style(style::button_primary)
                        .padding([8, 16]),
                ]
                .spacing(10)
            ]
            .spacing(20)
            .padding(30)
            .max_width(500),
        )
        .style(style::container_modal)
        .into()
    }

    fn view_simulation_modal<'a>(
        &'a self,
        report: &'a x_adox_core::scenery::validator::ValidationReport,
        simulated_packs: &'a Arc<Vec<SceneryPack>>,
    ) -> Element<'a, Message> {
        use x_adox_core::scenery::validator::ValidationSeverity;

        // Group issues by type
        let mut groups: std::collections::BTreeMap<
            &str,
            Vec<&x_adox_core::scenery::validator::ValidationIssue>,
        > = std::collections::BTreeMap::new();
        let mut visible_count = 0;

        for issue in &report.issues {
            if self
                .ignored_issues
                .contains(&(issue.issue_type.clone(), issue.pack_name.clone()))
            {
                continue;
            }
            groups.entry(&issue.issue_type).or_default().push(issue);
            visible_count += 1;
        }

        let issues_view: Element<'a, Message, Theme, Renderer> = if visible_count == 0 {
            container(
                column![
                    svg(self.icon_scenery.clone()) // Use scenery icon as placeholder or add a "check" icon
                        .width(Length::Fixed(48.0))
                        .height(Length::Fixed(48.0)),
                    text("All checks passed!")
                        .size(20)
                        .color(style::palette::ACCENT_GREEN),
                    text("Safe to apply these changes to scenery_packs.ini.")
                        .size(14)
                        .color(style::palette::TEXT_SECONDARY),
                ]
                .spacing(10)
                .align_x(iced::Alignment::Center),
            )
            .width(Length::Fill)
            .padding(40)
            .align_x(iced::alignment::Horizontal::Center)
            .into()
        } else {
            let mut content = Column::new().spacing(15);

            for (issue_type, issues) in groups {
                let is_expanded = self.expanded_issue_groups.contains(issue_type);
                let count = issues.len();
                let first = issues[0];

                let (icon_color, _) = match first.severity {
                    ValidationSeverity::Critical => {
                        (Color::from_rgb(1.0, 0.3, 0.3), style::button_card)
                    } // Highlight critical
                    ValidationSeverity::Warning => {
                        (style::palette::ACCENT_ORANGE, style::button_card)
                    }
                    ValidationSeverity::Info => {
                        (style::palette::TEXT_SECONDARY, style::button_card)
                    }
                };

                // Group Header / Compact Card
                let header = container(
                    column![
                        row![
                            svg(self.icon_warning.clone())
                                .width(Length::Fixed(20.0))
                                .height(Length::Fixed(20.0))
                                .style(move |_, _| iced::widget::svg::Style {
                                    color: Some(icon_color)
                                }),
                            tooltip(
                                {
                                    let base_msg = match issue_type {
                                        "shadowed_mesh" => "Redundant Mesh Scenery Detected",
                                        "simheaven_below_global" => "simHeaven Layers Misplaced",
                                        "mesh_above_overlay" => "Mesh/Overlay Layering Issues",
                                        _ => first.message.as_str(),
                                    };
                                    text(if count > 1 {
                                        format!("{} ({} affected)", base_msg, count)
                                    } else {
                                        first.message.clone()
                                    })
                                }
                                .size(16)
                                .color(icon_color),
                                text(&first.details).size(12),
                                tooltip::Position::Top,
                            ),
                        ]
                        .spacing(10)
                        .align_y(iced::Alignment::Center),
                        if count == 1 {
                            let pack_name = &first.pack_name;
                            let is_pinned = self
                                .heuristics_model
                                .config
                                .overrides
                                .contains_key(pack_name);
                            Element::from(container(
                                row![
                                    if is_pinned {
                                        Element::from(
                                            svg(self.icon_pin.clone())
                                                .width(Length::Fixed(12.0))
                                                .height(Length::Fixed(12.0))
                                                .style(move |_: &Theme, _| svg::Style {
                                                    color: Some(style::palette::ACCENT_RED),
                                                }),
                                        )
                                    } else {
                                        Element::from(iced::widget::Space::with_width(
                                            Length::Shrink,
                                        ))
                                    },
                                    text(pack_name.clone())
                                        .size(12)
                                        .color(style::palette::TEXT_SECONDARY),
                                ]
                                .spacing(5)
                                .align_y(iced::Alignment::Center),
                            ))
                        } else if !is_expanded {
                            Element::from(container(
                                text(format!("Click to expand {} affected packs", count))
                                    .size(12)
                                    .color(style::palette::TEXT_SECONDARY),
                            ))
                        } else {
                            Element::from(container(
                                text("Group Details")
                                    .size(12)
                                    .color(style::palette::TEXT_SECONDARY),
                            ))
                        }
                    ]
                    .spacing(5),
                )
                .width(Length::Fill);

                let mut group_col = Column::new().spacing(10);

                if count > 1 {
                    // Group with expansion logic
                    let toggle_btn = button(header)
                        .on_press(Message::ToggleIssueGroup(issue_type.to_string()))
                        .style(style::button_ghost)
                        .width(Length::Fill);

                    group_col = group_col.push(toggle_btn);

                    if is_expanded {
                        for issue in issues {
                            group_col = group_col.push(
                                container(
                                    row![
                                        text(&issue.pack_name)
                                            .size(13)
                                            .width(Length::Fill)
                                            .color(style::palette::TEXT_PRIMARY),
                                        button(text("Ignore").size(11))
                                            .on_press(Message::IgnoreIssue(
                                                issue.issue_type.clone(),
                                                issue.pack_name.clone()
                                            ))
                                            .style(style::button_secondary)
                                            .padding([4, 8]),
                                    ]
                                    .spacing(10)
                                    .align_y(iced::Alignment::Center),
                                )
                                .padding([5, 15])
                                .style(style::container_card),
                            );
                        }
                    }
                } else {
                    // Single issue card
                    group_col = group_col.push(
                        container(
                            column![
                                header,
                                text(&first.fix_suggestion)
                                    .size(13)
                                    .color(style::palette::TEXT_PRIMARY),
                                row![
                                    button(text("Auto-Fix").size(12))
                                        .on_press(Message::AutoFixIssue(issue_type.to_string()))
                                        .style(style::button_primary)
                                        .padding([6, 12]),
                                    button(text("Manual Edit").size(12))
                                        .on_press(Message::OpenHeuristicsEditor)
                                        .style(style::button_secondary)
                                        .padding([6, 12]),
                                    button(text("Ignore this").size(12))
                                        .on_press(Message::IgnoreIssue(
                                            first.issue_type.clone(),
                                            first.pack_name.clone()
                                        ))
                                        .style(style::button_secondary)
                                        .padding([6, 12]),
                                ]
                                .spacing(10)
                            ]
                            .spacing(10),
                        )
                        .padding(15)
                        .style(style::container_card),
                    );
                }

                // If expanded group has more than 1, add a global auto-fix button for the whole type
                if count > 1 && is_expanded {
                    group_col = group_col.push(
                        button(text(format!("Auto-Fix all {}", count)).size(12))
                            .on_press(Message::AutoFixIssue(issue_type.to_string()))
                            .style(style::button_primary)
                            .padding([8, 16]),
                    );
                }

                content = content.push(group_col);
            }
            content.into()
        };

        let preview: Column<'a, Message, Theme, Renderer> = column(
            simulated_packs
                .iter()
                .take(15) // Just show top 15 for preview
                .map(|p| {
                    row![
                        text(format!("{:?}", p.category))
                            .size(10)
                            .width(Length::Fixed(100.0)),
                        text(&p.name).size(12),
                    ]
                    .spacing(10)
                    .into()
                })
                .collect::<Vec<Element<'a, Message, Theme, Renderer>>>(),
        )
        .spacing(5);

        container(
            column![
                text("Smart Sort Simulation Report")
                    .size(24)
                    .color(Color::WHITE),
                text("Review potential issues before applying changes.")
                    .size(14)
                    .color(style::palette::TEXT_SECONDARY),
                scrollable(
                    Column::<Message, Theme, Renderer>::new()
                        .push(issues_view)
                        .push(
                            container(text("Resulting Order (Top 15):").size(18)).padding([10, 0])
                        )
                        .push(container(preview).padding(10).style(style::container_card))
                        .spacing(20)
                )
                .height(Length::Fill),
                row![
                    button(text("Cancel").size(14))
                        .on_press(Message::CancelSort)
                        .style(style::button_secondary)
                        .padding([10, 20]),
                    button(text("Apply Changes").size(14))
                        .on_press(Message::ApplySort(simulated_packs.clone()))
                        .style(style::button_premium_glow)
                        .padding([10, 30]),
                ]
                .spacing(20)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(20)
            .padding(30)
            .width(Length::Fixed(700.0)) // Wider for buttons
            .height(Length::Fixed(600.0)),
        )
        .style(style::container_card)
        .into()
    }

    fn view_utilities(&self) -> Element<'_, Message> {
        let logbook_header = button(
            row![
                text("Logbook").size(16),
                svg(self.icon_arrow_down.clone())
                    .width(14)
                    .height(14)
                    .style(move |_, _| svg::Style {
                        color: Some(Color::WHITE),
                    })
                    .rotation(iced::Radians(if self.logbook_expanded {
                        0.0
                    } else {
                        -std::f32::consts::PI / 2.0
                    })),
                iced::widget::horizontal_space(),
                text(format!("{} entries", self.logbook.len()))
                    .size(12)
                    .color(style::palette::TEXT_SECONDARY),
            ]
            .padding([0, 10])
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::ToggleLogbook)
        .style(style::button_secondary)
        .width(Length::Fill);

        let log_list_content: Element<'_, Message> = if !self.logbook_expanded {
            container(column![]).into()
        } else if self.logbook.is_empty() {
            container(text("No logbook entries found or logbook not loaded.").size(14))
                .center_x(Length::Fill)
                .padding(20)
                .into()
        } else {
            let mut col = Column::new().spacing(5);
            for (idx, entry) in self.logbook.iter().enumerate() {
                let is_selected = self.selected_flight == Some(idx);

                let date_str = entry
                    .date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "Unknown Date".to_string());
                let row_content = row![
                    text(date_str).width(Length::Fixed(100.0)).size(12),
                    text(&entry.dep_airport).width(Length::Fixed(60.0)).size(12),
                    text("->").width(Length::Fixed(20.0)).size(12),
                    text(&entry.arr_airport).width(Length::Fixed(60.0)).size(12),
                    text(&entry.aircraft_type)
                        .width(Length::Fixed(80.0))
                        .size(12),
                    text(format!("{:.1}h", entry.total_duration))
                        .width(Length::Fill)
                        .size(12),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center);

                let btn = button(row_content)
                    .on_press(Message::SelectFlight(Some(idx)))
                    .padding(8)
                    .width(Length::Fill)
                    .style(if is_selected {
                        style::button_primary
                    } else {
                        style::button_secondary
                    });

                col = col.push(btn);
            }
            scrollable(col).height(Length::Fill).into()
        };

        container(column![logbook_header, log_list_content,].spacing(10))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .style(style::container_card)
            .into()
    }

    fn view_navigator(&self) -> Element<'_, Message> {
        container(
            Column::<Message, Theme, Renderer>::new()
                .push(self.sidebar_button("Aircraft", Tab::Aircraft))
                .push(self.sidebar_button("Scenery", Tab::Scenery))
                .push(self.sidebar_button("Plugins", Tab::Plugins))
                .push(self.sidebar_button("CSLs", Tab::CSLs))
                .push(self.sidebar_button("Utilities", Tab::Utilities))
                .push(self.sidebar_button("Issues", Tab::Issues))
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
            Tab::Heuristics => self.view_heuristics_editor(),
            Tab::Issues => self.view_issues(),
            Tab::Utilities => self.view_utilities(),
            Tab::Settings => self.view_settings(),
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
            Tab::Heuristics => (Message::SaveHeuristics, Message::ResetHeuristics, true),
            Tab::Issues => (Message::CheckLogIssues, Message::Refresh, false),
            Tab::Utilities => (Message::Refresh, Message::Refresh, false),
            Tab::Settings => (Message::Refresh, Message::Refresh, false),
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

        let refresh_btn = button(
            row![
                svg(self.refresh_icon.clone())
                    .width(14)
                    .height(14)
                    .style(|_, _| svg::Style {
                        color: Some(Color::WHITE),
                    }),
                text("Refresh").size(12)
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::Refresh)
        .style(style::button_success)
        .padding([6, 12]);

        let smart_sort_btn =
            if self.active_tab == Tab::Scenery || self.active_tab == Tab::Heuristics {
                Some(
                    button(text("Smart Sort").size(12))
                        .on_press(Message::SmartSort)
                        .style(style::button_ai)
                        .padding([6, 12]),
                )
            } else {
                None
            };

        let mut actions = row![install_btn, delete_btn, refresh_btn].spacing(10);

        if self.active_tab == Tab::Aircraft && self.use_smart_view {
            let settings_btn = button(
                row![
                    svg(self.icon_settings.clone())
                        .width(14)
                        .height(14)
                        .style(|_, _| svg::Style {
                            color: Some(Color::WHITE),
                        }),
                    text("Settings").size(12)
                ]
                .spacing(6)
                .align_y(iced::Alignment::Center),
            )
            .on_press(Message::SwitchTab(Tab::Settings))
            .style(style::button_secondary)
            .padding([6, 12]);

            actions = actions.push(settings_btn);
        }
        if let Some(btn) = smart_sort_btn {
            actions = actions.push(btn);

            // Add Edit Sort button next to Smart Sort
            let edit_sort_btn = button(text("Edit Sort").size(12))
                .on_press(Message::OpenHeuristicsEditor)
                .style(style::button_premium_glow)
                .padding([6, 12]);
            actions = actions.push(edit_sort_btn);
        }

        let main_content = container(
            column![
                // Top Bar
                // Top Bar - Row 1: Path & Set Button
                // Path Selector Row (Top)
                // Actions & Profiles Row (Top)
                row![
                    actions,
                    iced::widget::horizontal_space().width(Length::Fixed(20.0)),
                    // Profile Selector (Phase 2)
                    if self.profile_manager.is_some() {
                        let options: Vec<String> = self
                            .profiles
                            .profiles
                            .iter()
                            .map(|p| p.name.clone())
                            .collect();
                        let selected = self.profiles.active_profile.clone();

                        row![
                            text("Profile:")
                                .size(12)
                                .color(style::palette::TEXT_SECONDARY),
                            pick_list(options, selected.clone(), Message::SwitchProfile)
                                .placeholder("Default")
                                .width(Length::Fixed(140.0))
                                .style(style::pick_list_primary)
                                .padding(4),
                            button(text("+").size(14))
                                .on_press(Message::OpenProfileDialog)
                                .style(style::button_secondary)
                                .padding([4, 8]),
                            // Rename Button
                            button(svg(self.icon_edit.clone()).width(14).height(14).style(
                                |_, _| svg::Style {
                                    color: Some(Color::WHITE),
                                }
                            ),)
                            .on_press(Message::OpenRenameDialog)
                            .style(style::button_secondary)
                            .padding([4, 8]),
                            // Delete Button
                            if let Some(name) = selected {
                                button(svg(self.icon_trash.clone()).width(14).height(14).style(
                                    |_, _| svg::Style {
                                        color: Some(Color::WHITE),
                                    },
                                ))
                                .on_press(Message::DeleteProfile(name))
                                .style(style::button_danger)
                                .padding([4, 8])
                            } else {
                                button(svg(self.icon_trash.clone()).width(14).height(14).style(
                                    |_, _| svg::Style {
                                        color: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.4)),
                                    },
                                ))
                                .style(style::button_secondary)
                                .padding([4, 8])
                            },
                        ]
                        .spacing(6)
                        .align_y(iced::Alignment::Center)
                    } else {
                        row![]
                    },
                    iced::widget::horizontal_space(),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center),
                // Path Selector Row (Bottom) - Dropdown for multiple installations
                {
                    let path_selector: Element<'_, Message, Theme, Renderer> =
                        if self.available_xplane_roots.is_empty() {
                            // No installations found, show placeholder text
                            text(path_text)
                                .size(12)
                                .color(style::palette::TEXT_SECONDARY)
                                .into()
                        } else {
                            // Show dropdown with all available installations
                            let options: Vec<String> = self
                                .available_xplane_roots
                                .iter()
                                .map(|p| p.to_string_lossy().to_string())
                                .collect();
                            let selected = self
                                .xplane_root
                                .as_ref()
                                .map(|p| p.to_string_lossy().to_string());
                            pick_list(options, selected, |s| {
                                Message::SelectXPlaneRoot(PathBuf::from(s))
                            })
                            .text_size(12)
                            .placeholder("Select X-Plane Installation")
                            .style(style::pick_list_primary)
                            .into()
                        };

                    row![
                        path_selector,
                        button(text("Browse...").size(12).color(Color::WHITE))
                            .on_press(Message::SelectFolder)
                            .style(style::button_secondary)
                            .padding([4, 8]),
                        text_input("Launch args (e.g. --no_plugins)", &self.launch_args)
                            .on_input(Message::LaunchArgsChanged)
                            .size(12)
                            .width(Length::Fixed(200.0))
                            .style(style::text_input_primary),
                        button(text("Launch").size(12).color(Color::WHITE))
                            .on_press(Message::LaunchXPlane)
                            .style(style::button_success)
                            .padding([4, 12]),
                        iced::widget::horizontal_space(),
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center)
                },
                if let Some(p) = self.install_progress {
                    let progress_col: Column<'_, Message, Theme, _> = column![
                        text(format!("Extracting... {:.0}%", p))
                            .size(10)
                            .color(style::palette::TEXT_SECONDARY),
                        progress_bar(0.0..=100.0, p)
                            .height(4)
                            .style(|_theme: &Theme| progress_bar::Style {
                                background: Background::Color(style::palette::SURFACE),
                                bar: Background::Color(style::palette::ACCENT_BLUE),
                                border: Border {
                                    radius: 2.0.into(),
                                    ..Default::default()
                                },
                            })
                    ]
                    .spacing(5)
                    .padding([10, 0]);

                    Element::from(progress_col)
                } else {
                    container(column![]).into()
                },
                content,
            ]
            .spacing(20)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::FillPortion(2))
        .style(style::container_main_content);

        if self.show_profile_dialog {
            stack![
                main_content,
                container(self.view_profile_dialog())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(iced::Length::Fill)
                    .center_y(iced::Length::Fill)
                    .style(|_theme: &Theme| container::Style {
                        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.6))),
                        ..Default::default()
                    })
            ]
            .into()
        } else if self.show_rename_dialog {
            stack![
                main_content,
                container(self.view_rename_dialog())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(iced::Length::Fill)
                    .center_y(iced::Length::Fill)
                    .style(|_theme: &Theme| container::Style {
                        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.6))),
                        ..Default::default()
                    })
            ]
            .into()
        } else {
            main_content.into()
        }
    }

    fn view_profile_dialog(&self) -> Element<'_, Message> {
        let content = column![
            text("Save New Profile").size(24),
            text("Enter a name for the current configuration")
                .size(14)
                .color(style::palette::TEXT_SECONDARY),
            text_input("Profile Name", &self.new_profile_name)
                .on_input(Message::NewProfileNameChanged)
                .on_submit(Message::SaveCurrentProfile(self.new_profile_name.clone()))
                .padding(10)
                .size(16)
                .style(style::text_input_primary),
            row![
                button(text("Cancel").size(14))
                    .on_press(Message::CloseProfileDialog)
                    .style(style::button_secondary)
                    .padding([10, 20]),
                button(text("Save Profile").size(14))
                    .on_press(Message::SaveCurrentProfile(self.new_profile_name.clone()))
                    .style(style::button_premium_glow)
                    .padding([10, 30]),
            ]
            .spacing(20)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(20)
        .padding(30)
        .width(Length::Fixed(400.0));

        container(content)
            .style(style::container_modal)
            .padding(20)
            .into()
    }

    fn view_rename_dialog(&self) -> Element<'_, Message> {
        let old_name = self.profiles.active_profile.clone().unwrap_or_default();
        let content = column![
            text("Rename Profile").size(24),
            text(format!("Enter a new name for '{}'", old_name))
                .size(14)
                .color(style::palette::TEXT_SECONDARY),
            text_input("New Name", &self.rename_profile_name)
                .on_input(Message::RenameProfileNameChanged)
                .on_submit(Message::RenameProfile(
                    old_name.clone(),
                    self.rename_profile_name.clone()
                ))
                .padding(10)
                .size(16)
                .style(style::text_input_primary),
            row![
                button(text("Cancel").size(14))
                    .on_press(Message::CloseProfileDialog)
                    .style(style::button_secondary)
                    .padding([10, 20]),
                button(text("Rename Profile").size(14))
                    .on_press(Message::RenameProfile(
                        old_name,
                        self.rename_profile_name.clone()
                    ))
                    .style(style::button_premium_glow)
                    .padding([10, 30]),
            ]
            .spacing(20)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(20)
        .padding(30)
        .width(Length::Fixed(400.0));

        container(content)
            .style(style::container_modal)
            .padding(20)
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
                airports: &self.airports,
                selected_flight: self.selected_flight.and_then(|idx| self.logbook.get(idx)),
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
            container(if self.active_tab == Tab::Heuristics {
                column![
                    text("Score Reference").size(18),
                    text("LOWER SCORE = HIGHER PRIORITY")
                        .size(10)
                        .color(style::palette::TEXT_SECONDARY),
                    text("10 - Airports (High Priority)").size(12),
                    text("20 - Global Airports").size(12),
                    text("30 - Overlays / SimHeaven").size(12),
                    text("40 - Landmarks / Default").size(12),
                    text("42 - AutoOrtho Overlays").size(12),
                    text("45 - Libraries").size(12),
                    text("50 - Ortho (Photos)").size(12),
                    text("60 - Meshes").size(12),
                    text("95 - AutoOrtho Base").size(12),
                ]
                .spacing(10)
            } else {
                column![
                    text("Inspector Panel").size(18),
                    container(if let Some(idx) = self.selected_flight {
                        if let Some(entry) = self.logbook.get(idx) {
                            column![
                                text(format!(
                                    "Flight: {} -> {}",
                                    entry.dep_airport, entry.arr_airport
                                ))
                                .size(16),
                                text(format!(
                                    "Date: {}",
                                    entry.date.map(|d| d.to_string()).unwrap_or_default()
                                )),
                                text(format!(
                                    "Aircraft: {} ({})",
                                    entry.aircraft_type, entry.tail_number
                                )),
                                text(format!("Duration: {:.1} hours", entry.total_duration)),
                                text(format!("Landings: {}", entry.landings)),
                            ]
                            .spacing(5)
                        } else {
                            column![text("None")]
                        }
                    } else if let Some(target_name) = self
                        .selected_scenery
                        .as_ref()
                        .or(self.hovered_scenery.as_ref())
                    {
                        if let Some(pack) = self.packs.iter().find(|p| &p.name == target_name) {
                            let tags_ui: Element<'_, Message> = if !pack.tags.is_empty() {
                                let r = row(pack
                                    .tags
                                    .iter()
                                    .map(|t| {
                                        container(
                                            row![
                                                text(t.clone()).size(12).color(Color::WHITE),
                                                button(text("×").size(12).color(Color::WHITE))
                                                    .on_press(Message::RemoveTag(
                                                        pack.name.clone(),
                                                        t.clone()
                                                    ))
                                                    .style(style::button_ghost)
                                                    .padding(0)
                                            ]
                                            .spacing(4)
                                            .align_y(iced::Alignment::Center),
                                        )
                                        .padding([4, 8])
                                        .style(|_| container::Style {
                                            background: Some(iced::Background::Color(
                                                Color::from_rgb(0.3, 0.3, 0.3),
                                            )),
                                            border: iced::Border {
                                                radius: 10.0.into(),
                                                ..Default::default()
                                            },
                                            ..Default::default()
                                        })
                                        .into()
                                    })
                                    .collect::<Vec<Element<'_, Message>>>())
                                .spacing(5)
                                .wrap();
                                Element::from(r)
                            } else {
                                text("No tags").size(10).into()
                            };

                            let add_ui: Element<'_, Message> =
                                if self.selected_scenery.as_ref() == Some(&pack.name) {
                                    let r = row![
                                        text_input("New Tag...", &self.new_tag_input)
                                            .on_input(Message::UpdateTagInput)
                                            .on_submit(Message::AddTag)
                                            .padding(6)
                                            .size(12)
                                            .width(Length::Fill),
                                        button(text("+").size(14))
                                            .on_press(Message::AddTag)
                                            .style(style::button_primary)
                                            .padding([6, 12])
                                    ]
                                    .spacing(5);
                                    Element::from(r)
                                } else {
                                    text("Select to edit")
                                        .size(10)
                                        .color(style::palette::TEXT_SECONDARY)
                                        .into()
                                };

                            column![
                                text(target_name).size(12).font(iced::Font::MONOSPACE),
                                text(format!(
                                    "CATEGORY: {:?} | TILES: {} | AIRPORTS: {}",
                                    pack.category,
                                    pack.tiles.len(),
                                    pack.airports.len()
                                ))
                                .size(10),
                                iced::widget::horizontal_rule(1.0),
                                text("TAGS").size(10).color(style::palette::TEXT_SECONDARY),
                                tags_ui,
                                add_ui
                            ]
                            .spacing(10)
                        } else {
                            column![text("Pack not found").size(12)].spacing(10)
                        }
                    } else {
                        column![text("None").size(12)].spacing(10)
                    })
                    .style(|_| container::Style::default())
                ]
                .spacing(10)
            })
            .style(style::container_card)
            .padding(15)
            .width(Length::Fixed(300.0))
            .height(Length::Fill)
        ]
        .spacing(20)
        .height(Length::Fill)
        .into()
    }

    fn resort_scenery(&mut self) {
        let context = x_adox_bitnet::PredictContext {
            region_focus: self.region_focus.clone(),
        };
        let model = &self.heuristics_model;

        Arc::make_mut(&mut self.packs).sort_by(|a, b| {
            let score_a = model.predict(&a.name, std::path::Path::new(""), &context);
            let score_b = model.predict(&b.name, std::path::Path::new(""), &context);
            score_a.cmp(&score_b)
        });
    }

    fn sidebar_button(&self, label: &'static str, tab: Tab) -> Element<'_, Message> {
        let is_active = self.active_tab == tab;

        let (icon_handle, active_color) = match tab {
            Tab::Aircraft => (&self.icon_aircraft, Color::WHITE),
            Tab::Scenery => (&self.icon_scenery, Color::from_rgb(0.4, 0.8, 0.4)), // Green
            Tab::Plugins => (&self.icon_plugins, Color::from_rgb(0.4, 0.6, 1.0)), // Blue
            Tab::CSLs => (&self.icon_csls, Color::from_rgb(1.0, 0.6, 0.2)),       // Orange
            Tab::Utilities => (&self.icon_utilities, Color::from_rgb(0.8, 0.5, 1.0)), // Purple
            Tab::Heuristics => (&self.refresh_icon, Color::from_rgb(0.8, 0.8, 0.8)), // Gray
            Tab::Issues => (&self.icon_warning, Color::from_rgb(1.0, 0.2, 0.2)), // Always red for Issues
            Tab::Settings => (&self.icon_settings, Color::from_rgb(0.7, 0.7, 0.7)), // Light gray for settings
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

        let has_issues = tab == Tab::Issues && !self.log_issues.is_empty();

        let icon_container = container(icon)
            .padding(5)
            .width(Length::Fixed(48.0))
            .height(Length::Fixed(48.0))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(move |_theme| {
                let mut shadow = Shadow::default();
                let mut background = None;

                if is_active {
                    background = Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.1)));
                    shadow = Shadow {
                        color: Color::from_rgba(
                            active_color.r,
                            active_color.g,
                            active_color.b,
                            0.4,
                        ),
                        offset: iced::Vector::new(0.0, 0.0),
                        blur_radius: 12.0,
                    };
                } else if has_issues {
                    background = Some(Background::Color(Color::from_rgba(1.0, 0.0, 0.0, 0.05)));
                    shadow = Shadow {
                        color: Color::from_rgba(1.0, 0.0, 0.0, 0.4),
                        offset: iced::Vector::new(0.0, 0.0),
                        blur_radius: 10.0,
                    };
                }

                container::Style {
                    background,
                    border: Border {
                        radius: 12.0.into(),
                        ..Default::default()
                    },
                    shadow,
                    ..Default::default()
                }
            });

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
                iced::widget::Space::new(Length::Fixed(4.0), Length::Fill), // Balancing spacer
                btn,
                column![
                    iced::widget::Space::new(Length::Fill, Length::Fixed(25.0)), // Explicit top offset
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
                .width(Length::Fixed(4.0))
            ]
            .align_y(iced::Alignment::Start)
            .into()
        } else {
            btn.into()
        }
    }

    fn view_scenery(&self) -> Element<'_, Message> {
        use iced::widget::lazy;

        let packs = self.packs.clone();
        let selected = self.selected_scenery.clone();
        let hovered = self.hovered_scenery.clone();
        let overrides = self.heuristics_model.config.overrides.clone();

        // Icons needed for cards
        let icons = (
            self.icon_pin.clone(),
            self.icon_pin_outline.clone(),
            self.icon_arrow_up.clone(),
            self.icon_arrow_down.clone(),
        );

        let list_container = scrollable(lazy(
            (packs, selected, hovered, overrides),
            move |(packs, selected, _hovered, overrides)| {
                let cards: Vec<Element<'static, Message, Theme, Renderer>> = packs
                    .iter()
                    .map(|pack| {
                        Self::render_scenery_card(
                            pack,
                            selected.as_ref() == Some(&pack.name),
                            overrides.contains_key(&pack.name),
                            icons.clone(),
                        )
                    })
                    .collect();

                Element::from(column(cards).spacing(10))
            },
        ))
        .id(self.scenery_scroll_id.clone());

        column![
            row![
                text("Scenery Library").size(24).width(Length::Fill),
                if !self.heuristics_model.config.overrides.is_empty() {
                    let btn: Element<'_, Message> =
                        button(
                            Row::<Message, Theme, Renderer>::new()
                                .push(svg(self.icon_pin.clone()).width(14).height(14).style(
                                    |_, _| svg::Style {
                                        color: Some(style::palette::ACCENT_RED),
                                    },
                                ))
                                .push(
                                    text(format!(
                                        "Clear All Pins ({})",
                                        self.heuristics_model.config.overrides.len()
                                    ))
                                    .size(12),
                                )
                                .spacing(8)
                                .align_y(iced::Alignment::Center),
                        )
                        .on_press(Message::ClearAllPins)
                        .style(style::button_secondary)
                        .padding([6, 12])
                        .into();
                    btn
                } else {
                    iced::widget::Space::new(Length::Fixed(0.0), Length::Fixed(0.0)).into()
                }
            ]
            .align_y(iced::Alignment::Center)
            .padding(10),
            list_container
        ]
        .spacing(10)
        .into()
    }

    fn view_heuristics_editor(&self) -> Element<'_, Message> {
        let editor = text_editor(&self.heuristics_json)
            .on_action(Message::HeuristicsAction)
            .font(iced::Font::MONOSPACE);

        let error_banner = if let Some(err) = &self.heuristics_error {
            container(text(err).color(Color::from_rgb(1.0, 0.3, 0.3)))
                .padding(10)
                .style(style::container_card)
        } else {
            container(column![])
        };

        let toolbar = row![
            button(text("Save Rules").size(14))
                .on_press(Message::SaveHeuristics)
                .style(style::button_primary)
                .padding([10, 20]),
            button(text("Import").size(14))
                .on_press(Message::ImportHeuristics)
                .style(style::button_secondary)
                .padding([10, 20]),
            button(text("Export").size(14))
                .on_press(Message::ExportHeuristics)
                .style(style::button_secondary)
                .padding([10, 20]),
            button(text("Reset to Defaults").size(14))
                .on_press(Message::ResetHeuristics)
                .style(style::button_secondary)
                .padding([10, 20]),
        ]
        .spacing(15);

        container(
            column![
                text("Scenery Sorting Heuristics (JSON Editor)")
                    .size(20)
                    .width(Length::Fill),
                text("Customize the weights and keywords used by the BitNet AI for sorting.")
                    .size(14)
                    .color(Color::from_rgb(0.6, 0.6, 0.6)),
                row![
                    text("Region Focus:").size(14),
                    row(vec!["America", "Europe", "Asia", "Australia", "Africa"]
                        .into_iter()
                        .map(|r| {
                            let is_selected = self.region_focus.as_deref() == Some(r);
                            button(text(r).size(12))
                                .on_press(Message::SetRegionFocus(if is_selected {
                                    None
                                } else {
                                    Some(r.to_string())
                                }))
                                .style(if is_selected {
                                    style::button_primary
                                } else {
                                    style::button_secondary
                                })
                                .padding([5, 10])
                                .into()
                        })
                        .collect::<Vec<Element<'_, Message>>>())
                    .spacing(10),
                ]
                .spacing(20)
                .align_y(iced::Alignment::Center),
                error_banner,
                container(editor)
                    .height(Length::Fill)
                    .width(Length::Fill)
                    .style(style::container_card)
                    .padding(5),
                toolbar,
            ]
            .spacing(15)
            .padding(20),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(style::container_main_content)
        .into()
    }

    fn view_settings(&self) -> Element<'_, Message> {
        let title = text("Scan Settings").size(24);

        // Exclusions Section
        let exclusions_title = text("Excluded Folders (Ignored by AI Scan)").size(18);

        let exclusions_list: Element<'_, Message> = if self.scan_exclusions.is_empty() {
            text("No folders excluded.")
                .color(style::palette::TEXT_SECONDARY)
                .into()
        } else {
            column(
                self.scan_exclusions
                    .iter()
                    .map(|path| {
                        let p = path.clone();
                        container(
                            row![
                                text(path.to_string_lossy()).width(Length::Fill),
                                button(text("Remove").size(12))
                                    .on_press(Message::RemoveExclusion(p))
                                    .style(style::button_danger)
                                    .padding([5, 10])
                            ]
                            .align_y(iced::Alignment::Center)
                            .spacing(10),
                        )
                        .padding(10)
                        .style(style::container_card)
                        .into()
                    })
                    .collect::<Vec<_>>(),
            )
            .spacing(5)
            .into()
        };

        let add_btn = button(
            row![
                svg(self.icon_plugins.clone()).width(Length::Fixed(16.0)), // reusing plugins icon as generic 'plus' or folder
                text("Add Exclusion Folder")
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::AddExclusion)
        .padding(10)
        .style(style::button_primary);

        column![
            row![
                button(text("Back").size(12))
                    .on_press(Message::SwitchTab(Tab::Aircraft))
                    .style(style::button_secondary)
                    .padding([5, 10]),
                title
            ]
            .spacing(20)
            .align_y(iced::Alignment::Center),
            container(
                column![
                    exclusions_title,
                    text("Changes require a refresh to take effect.")
                        .size(12)
                        .color(style::palette::TEXT_SECONDARY),
                    add_btn,
                    exclusions_list
                ]
                .spacing(20)
            )
            .padding(20)
            .style(style::container_card)
            .width(Length::Fill)
        ]
        .spacing(20)
        .padding(20)
        .into()
    }

    fn view_issues(&self) -> Element<'_, Message> {
        let title = text("Issues Dashboard")
            .size(32)
            .color(style::palette::ACCENT_ORANGE);

        let mut content = column![title].spacing(30);

        // 1. Log Issues Section
        content = content.push(text("X-Plane Log Analysis").size(24));

        let log_content: Element<'_, Message> = if self.log_issues.is_empty() {
            container(
                column![
                    text("No issues detected in Log.txt").size(16),
                    button("Re-scan Log")
                        .padding([8, 16])
                        .style(style::button_primary)
                        .on_press(Message::CheckLogIssues),
                ]
                .spacing(10),
            )
            .padding(20)
            .style(style::container_card)
            .width(Length::Fill)
            .into()
        } else {
            let issues_list = column(self.log_issues.iter().map(|issue| {
                container(
                    column![
                        row![
                            text("Missing Resource: ").color(style::palette::TEXT_SECONDARY),
                            text(&issue.resource_path).color(style::palette::TEXT_PRIMARY),
                        ]
                        .spacing(5),
                        row![
                            text("Referenced from: ").color(style::palette::TEXT_SECONDARY),
                            text(&issue.package_path).color(style::palette::TEXT_PRIMARY),
                        ]
                        .spacing(5),
                        if let Some(lib) = &issue.potential_library {
                            row![
                                text("Potential Library: ").color(style::palette::TEXT_SECONDARY),
                                text(lib).color(style::palette::ACCENT_BLUE),
                            ]
                            .spacing(5)
                        } else {
                            row![].into()
                        },
                    ]
                    .spacing(8),
                )
                .padding(15)
                .style(style::container_card)
                .width(Length::Fill)
                .into()
            }))
            .spacing(10);

            column![
                text(format!(
                    "Found {} missing resources.",
                    self.log_issues.len()
                ))
                .color(style::palette::ACCENT_RED),
                issues_list,
                button("Re-scan Log")
                    .on_press(Message::CheckLogIssues)
                    .style(style::button_secondary)
            ]
            .spacing(10)
            .into()
        };
        content = content.push(log_content);

        content = content.push(iced::widget::horizontal_rule(1.0));

        // 2. Validation Issues Section
        content = content.push(text("Scenery Order Validation").size(24));

        let validation_content = if let Some(report) = &self.validation_report {
            if report.issues.is_empty() {
                let c = container(
                    text("No validation issues found. Scenery order looks good!")
                        .size(16)
                        .color(style::palette::ACCENT_GREEN),
                )
                .padding(20)
                .style(style::container_card)
                .width(Length::Fill);
                Element::from(c)
            } else {
                let issues = column(report.issues.iter().map(|issue| {
                    let c = container(
                        column![
                            row![
                                text(issue.issue_type.to_uppercase())
                                    .size(12)
                                    .font(iced::Font::MONOSPACE)
                                    .color(style::palette::ACCENT_RED),
                                text(&issue.pack_name).size(14).font(iced::Font::MONOSPACE)
                            ]
                            .spacing(10),
                            text(&issue.message).size(14),
                            text(format!("Fix: {}", issue.fix_suggestion))
                                .size(12)
                                .color(style::palette::ACCENT_BLUE),
                            text(&issue.details)
                                .size(10)
                                .color(style::palette::TEXT_SECONDARY)
                        ]
                        .spacing(5),
                    )
                    .padding(15)
                    .style(style::container_card)
                    .width(Length::Fill);
                    Element::from(c)
                }))
                .spacing(10);

                column![
                    text(format!("Found {} validation issues.", report.issues.len()))
                        .color(style::palette::ACCENT_RED),
                    issues
                ]
                .spacing(10)
                .into()
            }
        } else {
            container(
                column![
                    text("Validation not run yet.").size(16),
                    text("Run 'Smart Sort/Simulate' to generate a validation report.")
                        .size(12)
                        .color(style::palette::TEXT_SECONDARY)
                ]
                .spacing(10),
            )
            .padding(20)
            .style(style::container_card)
            .width(Length::Fill)
            .into()
        };
        content = content.push(validation_content);

        container(scrollable(content).height(Length::Fill))
            .padding(30)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(style::container_main_content)
            .into()
    }

    fn render_scenery_card(
        pack: &SceneryPack,
        is_selected: bool,
        is_heroic: bool,
        icons: (
            iced::widget::svg::Handle,
            iced::widget::svg::Handle,
            iced::widget::svg::Handle,
            iced::widget::svg::Handle,
        ),
    ) -> Element<'static, Message> {
        let is_active = pack.status == SceneryPackType::Active;
        let (icon_pin, icon_pin_outline, icon_arrow_up, icon_arrow_down) = icons;

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

        let name_text = text(pack.name.clone())
            .size(14)
            .color(style::palette::TEXT_PRIMARY);
        let sub_text = text(if is_active { "Active" } else { "Disabled" })
            .size(10)
            .color(style::palette::TEXT_SECONDARY);

        let info_col = column![name_text, sub_text].spacing(4).width(Length::Fill);

        let tag_color = match pack.category {
            x_adox_core::scenery::SceneryCategory::EarthAirports => style::palette::ACCENT_ORANGE,
            x_adox_core::scenery::SceneryCategory::Library => style::palette::ACCENT_BLUE,
            _ => style::palette::TEXT_SECONDARY,
        };

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

        let tags_row = row(pack
            .tags
            .iter()
            .map(|t| {
                container(text(t.clone()).size(9).color(Color::WHITE))
                    .padding([2, 5])
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgb(0.4, 0.4, 0.4))),
                        border: iced::Border {
                            radius: 3.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .into()
            })
            .collect::<Vec<Element<'static, Message>>>())
        .spacing(4);

        let pin_btn = if is_heroic {
            button(
                svg(icon_pin.clone())
                    .width(Length::Fixed(16.0))
                    .height(Length::Fixed(16.0))
                    .style(move |_: &Theme, _| svg::Style {
                        color: Some(style::palette::ACCENT_RED),
                    }),
            )
            .on_press(Message::RemovePriority(pack.name.clone()))
            .style(style::button_pin_active)
        } else {
            button(
                svg(icon_pin_outline.clone())
                    .width(Length::Fixed(16.0))
                    .height(Length::Fixed(16.0))
                    .style(move |_: &Theme, _| svg::Style {
                        color: Some(Color::from_rgba(0.93, 0.25, 0.25, 0.6)),
                    }),
            )
            .on_press(Message::OpenPriorityEditor(pack.name.clone()))
            .style(style::button_pin_ghost)
        };

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

        let move_up_btn = button(
            svg(icon_arrow_up.clone())
                .width(Length::Fixed(12.0))
                .height(Length::Fixed(12.0))
                .style(move |_: &Theme, _| svg::Style {
                    color: Some(style::palette::TEXT_SECONDARY),
                }),
        )
        .on_press(Message::MovePack(pack.name.clone(), MoveDirection::Up))
        .style(style::button_ghost)
        .padding(4);

        let move_down_btn = button(
            svg(icon_arrow_down.clone())
                .width(Length::Fixed(12.0))
                .height(Length::Fixed(12.0))
                .style(move |_: &Theme, _| svg::Style {
                    color: Some(style::palette::TEXT_SECONDARY),
                }),
        )
        .on_press(Message::MovePack(pack.name.clone(), MoveDirection::Down))
        .style(style::button_ghost)
        .padding(4);

        let move_controls = column![move_up_btn, move_down_btn].spacing(2);

        let content_row = row![
            status_dot,
            info_col,
            tags_row,
            type_tag,
            move_controls,
            pin_btn,
            action_btn
        ]
        .spacing(15)
        .align_y(iced::Alignment::Center);

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
        addons: &'a Arc<Vec<DiscoveredAddon>>,
        label: &str,
    ) -> Element<'a, Message> {
        use iced::widget::lazy;

        let label_owned = label.to_string();
        let active_tab = self.active_tab;
        let selected_path = match label {
            "Plugin" => self.selected_plugin.clone(),
            "CSL Package" => self.selected_csl.clone(),
            _ => self.selected_aircraft.clone(),
        };
        let show_delete_confirm = self.show_delete_confirm;

        lazy(
            (
                addons.clone(),
                selected_path.clone(),
                show_delete_confirm,
                active_tab,
                label_owned,
            ),
            move |(addons, selected_path, show_delete_confirm, active_tab, label)| {
                let is_plugins = label == "Plugin";
                let is_csls = label == "CSL Package";

                let confirm_text = if *show_delete_confirm
                    && ((is_plugins && *active_tab == Tab::Plugins)
                        || (!is_plugins && !is_csls && *active_tab == Tab::Aircraft)
                        || (is_csls && *active_tab == Tab::CSLs))
                {
                    if let Some(ref path) = selected_path {
                        Some(format!("Delete {} at '{}'?", label, path.display()))
                    } else {
                        None
                    }
                } else {
                    None
                };

                let list_content: Element<'_, Message, Theme, Renderer> = if addons.is_empty() {
                    Element::from(text(format!("No {} found", label)).size(14))
                } else {
                    let list: Column<Message, Theme, Renderer> =
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
                            let row_content: Element<'_, Message, Theme, Renderer> = if is_csls
                                || is_plugins
                            {
                                let is_enabled = addon.is_enabled;
                                let path_for_toggle = path.clone();

                                row![
                                    checkbox("", is_enabled)
                                        .on_toggle(move |e| if is_plugins {
                                            Message::TogglePlugin(path_for_toggle.clone(), e)
                                        } else {
                                            Message::ToggleCSL(path_for_toggle.clone(), e)
                                        })
                                        .text_size(14),
                                    button(text(addon.name.clone()).size(14).width(Length::Fill))
                                        .on_press(if is_plugins {
                                            Message::SelectPlugin(path.clone())
                                        } else {
                                            Message::SelectCSL(path.clone())
                                        })
                                        .style(style)
                                        .padding([4, 8])
                                        .width(Length::Fill),
                                ]
                                .spacing(5)
                                .into()
                            } else {
                                button(
                                    row![
                                        text(addon.name.clone()).size(14).width(Length::Fill),
                                        text(type_label.clone())
                                            .size(12)
                                            .color(style::palette::TEXT_SECONDARY),
                                    ]
                                    .spacing(10)
                                    .align_y(iced::Alignment::Center),
                                )
                                .on_press(Message::SelectAircraft(path))
                                .style(style)
                                .padding([4, 8])
                                .width(Length::Fill)
                                .into()
                            };

                            col.push(row_content)
                        });

                    scrollable(list)
                        .height(Length::Fill)
                        .width(Length::Fill)
                        .into()
                };

                let main_content = column![list_content].spacing(10);

                if let Some(confirm_msg) = confirm_text {
                    Element::from(
                        column![
                            main_content,
                            row![
                                text(confirm_msg.clone()).size(14),
                                button("Yes, Delete")
                                    .on_press(Message::ConfirmDelete(
                                        if is_plugins {
                                            Tab::Plugins
                                        } else if is_csls {
                                            Tab::CSLs
                                        } else {
                                            Tab::Aircraft
                                        },
                                        true,
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
                                        false,
                                    ))
                                    .padding([6, 12]),
                            ]
                            .spacing(10)
                            .align_y(iced::Alignment::Center)
                        ]
                        .spacing(10),
                    )
                } else {
                    Element::from(main_content)
                }
            },
        )
        .into()
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

        let toggle_view = button(
            text(if self.use_smart_view {
                "Folder View"
            } else {
                "AI Smart View"
            })
            .size(12),
        )
        .on_press(Message::ToggleAircraftSmartView)
        .style(style::button_secondary)
        .padding([4, 8]);

        let items = if self.use_smart_view {
            Self::collect_smart_nodes(
                &self.aircraft_tree,
                &self.selected_aircraft,
                &self.smart_view_expanded,
            )
        } else {
            match &self.aircraft_tree {
                Some(tree) => Self::collect_tree_nodes(tree, 0, &self.selected_aircraft),
                None => vec![Element::from(text("Loading aircraft...").size(14))],
            }
        };

        use iced::widget::lazy;
        let tree_content = if items.is_empty() && self.aircraft_tree.is_some() {
            Element::from(text("No aircraft found.").size(14))
        } else {
            let use_smart = self.use_smart_view;
            let tree = self.aircraft_tree.clone();
            let selected = self.selected_aircraft.clone();

            scrollable(lazy(
                (
                    use_smart,
                    tree.clone(),
                    selected.clone(),
                    self.smart_view_expanded.clone(),
                ),
                move |(use_smart, tree, selected, expanded)| {
                    let items: Vec<Element<'_, Message, Theme, Renderer>> = if *use_smart {
                        Self::collect_smart_nodes(&tree, &selected, &expanded)
                    } else {
                        match tree {
                            Some(t) => Self::collect_tree_nodes(t, 0, selected),
                            None => vec![Element::from(text("Loading aircraft...").size(14))],
                        }
                    };
                    Element::from(column(items).spacing(2))
                },
            ))
            .height(Length::Fill)
            .into()
        };

        let list_pane = column![
            row![text("Aircraft Library").size(18), toggle_view]
                .spacing(10)
                .align_y(iced::Alignment::Center)
                .padding(iced::Padding {
                    top: 0.0,
                    right: 0.0,
                    bottom: 10.0,
                    left: 0.0,
                }),
            tree_content
        ];

        let preview: Element<'_, Message> = if let Some(icon) = &self.selected_aircraft_icon {
            let tags_row = row(self
                .selected_aircraft_tags
                .iter()
                .map(|t| {
                    container(text(t).size(12).color(style::palette::TEXT_PRIMARY))
                        .padding([4, 8])
                        .style(style::container_card)
                        .into()
                })
                .collect::<Vec<_>>())
            .spacing(5)
            .wrap();

            let category_selector: Element<'_, Message> =
                if let Some(name) = &self.selected_aircraft_name {
                    let options: Vec<String> =
                        AIRCRAFT_CATEGORIES.iter().map(|&s| s.to_string()).collect();
                    column![
                        text("Set AI Category Manually:")
                            .size(12)
                            .color(style::palette::TEXT_SECONDARY),
                        pick_list(options, None::<String>, move |selected| {
                            Message::SetAircraftCategory(name.clone(), selected)
                        })
                        .placeholder("Choose override...")
                        .padding(8)
                        .width(Length::Fill),
                    ]
                    .spacing(5)
                    .into()
                } else {
                    column![].into()
                };

            let change_icon_btn: Element<'_, Message> = if let Some(path) = &self.selected_aircraft
            {
                let p = path.clone();
                button(text("Change Icon").size(12))
                    .on_press(Message::BrowseForIcon(p))
                    .style(style::button_secondary)
                    .padding([5, 10])
                    .into()
            } else {
                column![].into()
            };

            container(
                column![
                    iced::widget::image(icon.clone()),
                    change_icon_btn,
                    tags_row,
                    category_selector
                ]
                .spacing(20)
                .align_x(iced::Alignment::Center),
            )
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

        let main_content =
            row![container(list_pane).width(Length::FillPortion(2)), preview].spacing(20);

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

    fn collect_tree_nodes(
        node: &AircraftNode,
        depth: usize,
        selected_aircraft: &Option<std::path::PathBuf>,
    ) -> Vec<Element<'static, Message>> {
        let mut result = vec![Self::render_aircraft_row(node, depth, selected_aircraft)];

        // Collect children if expanded
        if node.is_expanded {
            for child in &node.children {
                result.extend(Self::collect_tree_nodes(
                    child,
                    depth + 1,
                    selected_aircraft,
                ));
            }
        }

        result
    }

    fn render_aircraft_row(
        node: &AircraftNode,
        depth: usize,
        selected_aircraft: &Option<std::path::PathBuf>,
    ) -> Element<'static, Message> {
        let indent = 20.0 * depth as f32;

        // Determine icon based on node type
        let icon = if node.is_folder {
            if node.is_expanded {
                "v"
            } else {
                ">"
            }
        } else if node.acf_file.is_some() {
            "   •"
        } else {
            "   -"
        };

        let display_name = if let Some(acf) = &node.acf_file {
            format!("{} ({})", node.name, acf)
        } else {
            node.name.clone()
        };

        let is_selected = selected_aircraft.as_ref() == Some(&node.path);
        let style = if is_selected {
            button::primary
        } else {
            style::button_ghost
        };

        let label_color = if node.is_enabled {
            style::palette::TEXT_PRIMARY
        } else {
            style::palette::TEXT_SECONDARY // Dimmed for disabled
        };

        let node_row: Element<'static, Message> = if node.is_folder {
            let path = node.path.clone();
            let path_for_select = node.path.clone();

            row![
                button(text(icon).size(14))
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
            let toggle_path = node.path.clone();
            let is_enabled = node.is_enabled;

            // Allow toggling only if it's an aircraft package (leaf node with .acf usually)
            let toggle_btn: Element<'static, Message> = if node.acf_file.is_some() {
                checkbox("Enabled", is_enabled)
                    .on_toggle(move |v| Message::ToggleAircraft(toggle_path.clone(), v))
                    .size(16)
                    .spacing(10)
                    .into()
            } else {
                iced::widget::Space::with_width(Length::Fixed(26.0)).into()
            };

            row![
                toggle_btn,
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
            ]
            .spacing(10)
            .into()
        };

        row![container(text("")).width(Length::Fixed(indent)), node_row,].into()
    }

    fn flatten_aircraft_tree(node: &AircraftNode) -> Vec<AircraftNode> {
        let mut result = Vec::new();
        if node.acf_file.is_some() {
            result.push(node.clone());
        }
        for child in &node.children {
            result.extend(Self::flatten_aircraft_tree(child));
        }
        result
    }

    fn collect_smart_nodes(
        tree: &Option<Arc<AircraftNode>>,
        selected_aircraft: &Option<std::path::PathBuf>,
        expanded_smart: &std::collections::BTreeSet<String>,
    ) -> Vec<Element<'static, Message>> {
        use std::collections::BTreeMap;
        let Some(tree) = tree else {
            return Vec::new();
        };
        let all_aircraft = Self::flatten_aircraft_tree(tree);

        let mut groups: BTreeMap<String, Vec<AircraftNode>> = BTreeMap::new();
        for ac in all_aircraft {
            let mut tags = ac.tags.clone();

            // Redundancy Reduction: If manufacturer present, remove "Airliner"
            let has_manufacturer = tags.iter().any(|t| MANUFACTURERS.contains(&t.as_str()));
            if has_manufacturer {
                tags.retain(|t| t != "Airliner");
            }

            if tags.is_empty() {
                groups.entry("Other".to_string()).or_default().push(ac);
            } else {
                for tag in &tags {
                    groups.entry(tag.clone()).or_default().push(ac.clone());
                }
            }
        }

        let mut result = Vec::new();
        for (tag, aircraft) in groups {
            let tag_id = format!("tag:{}", tag);
            let is_expanded = expanded_smart.contains(&tag_id);
            let icon = if is_expanded { "v" } else { ">" };

            result.push(
                row![
                    button(text(icon).size(14))
                        .on_press(Message::ToggleSmartFolder(tag_id))
                        .padding([4, 8])
                        .style(style::button_ghost),
                    container(text(tag.clone()).size(16)).padding(iced::Padding {
                        top: 5.0,
                        right: 5.0,
                        bottom: 5.0,
                        left: 5.0,
                    })
                ]
                .align_y(iced::Alignment::Center)
                .into(),
            );

            if !is_expanded {
                continue;
            }

            let is_manufacturer = MANUFACTURERS.contains(&tag.as_str());

            if is_manufacturer {
                // Group by model
                let mut model_groups: BTreeMap<String, Vec<AircraftNode>> = BTreeMap::new();
                for ac in aircraft {
                    let raw_model = ac
                        .acf_file
                        .as_ref()
                        .map(|f| f.strip_suffix(".acf").unwrap_or(f).to_string())
                        .unwrap_or_else(|| ac.name.clone());
                    let model = Self::normalize_model_name(&raw_model);
                    model_groups.entry(model).or_default().push(ac);
                }

                for (model, acs) in model_groups {
                    let model_id = format!("model:{}:{}", tag, model);
                    let model_expanded = expanded_smart.contains(&model_id);

                    // Multiple entries or single entry: ALWAYS folder for Model
                    let folder = AircraftNode {
                        name: model.clone(),
                        path: PathBuf::new(), // dummy path
                        is_folder: true,
                        is_expanded: model_expanded,
                        children: Vec::new(),
                        acf_file: None,
                        is_enabled: acs.iter().any(|ac| ac.is_enabled),
                        tags: Vec::new(),
                    };

                    // Use a custom row for virtual folder to support ToggleSmartFolder
                    let indent = 20.0;
                    let icon = if model_expanded { "v" } else { ">" };
                    let label_color = if folder.is_enabled {
                        style::palette::TEXT_PRIMARY
                    } else {
                        style::palette::TEXT_SECONDARY
                    };

                    result.push(
                        row![
                            container(text("")).width(Length::Fixed(indent)),
                            row![
                                button(text(icon).size(14))
                                    .on_press(Message::ToggleSmartFolder(model_id))
                                    .padding([4, 8])
                                    .style(style::button_ghost),
                                button(text(folder.name).size(14).color(label_color))
                                    .style(style::button_ghost)
                                    .padding([4, 8])
                            ]
                            .spacing(5)
                        ]
                        .into(),
                    );

                    if model_expanded {
                        for mut ac in acs {
                            // In the folder, show the identifier (usually airline)
                            ac.acf_file = None; // Hide (acf) because it's in the folder
                            result.push(Self::render_aircraft_row(&ac, 2, selected_aircraft));
                        }
                    }
                }
            } else {
                for ac in aircraft {
                    result.push(Self::render_aircraft_row(&ac, 1, selected_aircraft));
                }
            }
        }
        result
    }

    fn find_tags_in_tree(node: &AircraftNode, target: &std::path::Path) -> Option<Vec<String>> {
        if node.path == target {
            return Some(node.tags.clone());
        }
        for child in &node.children {
            if let Some(tags) = Self::find_tags_in_tree(child, target) {
                return Some(tags);
            }
        }
        None
    }

    fn normalize_model_name(raw_name: &str) -> String {
        let upper = raw_name.to_uppercase();

        // Airbus Heuristics
        // A300 and A306 (Beluga) -> A300
        if upper.starts_with("A300") || upper.starts_with("A306") {
            return "A300".to_string();
        }
        if upper.starts_with("A310") {
            return "A310".to_string();
        }
        // A320 Family - keep distinct but clean
        if upper.starts_with("A318") {
            return "A318".to_string();
        }
        if upper.starts_with("A319") {
            return "A319".to_string();
        }
        if upper.starts_with("A320") {
            return "A320".to_string();
        }
        if upper.starts_with("A321") {
            return "A321".to_string();
        }
        if upper.starts_with("A330") {
            return "A330".to_string();
        }
        if upper.starts_with("A340") {
            return "A340".to_string();
        }
        if upper.starts_with("A350") {
            return "A350".to_string();
        }
        if upper.starts_with("A380") {
            return "A380".to_string();
        }

        // Boeing Heuristics
        // Check for 7x7 pattern with optional B prefix
        // 737 (checking for 73x variants)
        // 737, 732, 733, 734, 735, 736, 738, 739
        // Also B737, B738, etc.
        if (upper.contains("737")
            || upper.contains("738")
            || upper.contains("739")
            || upper.contains("732")
            || upper.contains("733")
            || upper.contains("734")
            || upper.contains("735")
            || upper.contains("736"))
            && (upper.contains("B") || upper.starts_with("7"))
        {
            return "737".to_string();
        }
        // 747
        if upper.contains("747") && (upper.contains("B") || upper.starts_with("7")) {
            return "747".to_string();
        }
        // 757
        if upper.contains("757") && (upper.contains("B") || upper.starts_with("7")) {
            return "757".to_string();
        }
        // 767
        if upper.contains("767") && (upper.contains("B") || upper.starts_with("7")) {
            return "767".to_string();
        }
        // 777
        if upper.contains("777") && (upper.contains("B") || upper.starts_with("7")) {
            return "777".to_string();
        }
        // 787
        if upper.contains("787") && (upper.contains("B") || upper.starts_with("7")) {
            return "787".to_string();
        }

        // Default: return raw name as is (preserving case if not matched)
        raw_name.to_string()
    }

    fn get_icon_overrides_path() -> Option<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "startux", "x-adox")?;
        let config_dir = proj_dirs.config_dir();
        if !config_dir.exists() {
            let _ = std::fs::create_dir_all(config_dir);
        }
        Some(config_dir.join("icon_overrides.json"))
    }

    fn load_icon_overrides(&mut self) {
        if let Some(path) = Self::get_icon_overrides_path() {
            if let Ok(file) = std::fs::File::open(path) {
                let reader = std::io::BufReader::new(file);
                if let Ok(overrides) = serde_json::from_reader(reader) {
                    self.icon_overrides = overrides;
                    println!("Loaded {} icon overrides", self.icon_overrides.len());
                }
            }
        }
    }

    fn save_icon_overrides(&self) {
        if let Some(path) = Self::get_icon_overrides_path() {
            if let Ok(file) = std::fs::File::create(path) {
                let writer = std::io::BufWriter::new(file);
                let _ = serde_json::to_writer_pretty(writer, &self.icon_overrides);
            }
        }
    }

    fn get_scan_config_path() -> Option<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "startux", "x-adox")?;
        let config_dir = proj_dirs.config_dir();
        if !config_dir.exists() {
            let _ = std::fs::create_dir_all(config_dir);
        }
        Some(config_dir.join("scan_config.json"))
    }

    fn load_scan_config(&mut self) {
        if let Some(path) = Self::get_scan_config_path() {
            if let Ok(file) = std::fs::File::open(path) {
                let reader = std::io::BufReader::new(file);
                // Define a struct for serialization matching what we want
                #[derive(serde::Deserialize)]
                struct ScanConfig {
                    exclusions: Vec<PathBuf>,
                    #[serde(default)]
                    inclusions: Vec<PathBuf>,
                }

                if let Ok(config) = serde_json::from_reader::<_, ScanConfig>(reader) {
                    self.scan_exclusions = config
                        .exclusions
                        .into_iter()
                        .map(|p| p.canonicalize().unwrap_or(p))
                        .collect();
                    self.scan_inclusions = config.inclusions;
                    println!("Loaded {} excluded paths", self.scan_exclusions.len());
                }
            }
        }
    }

    fn save_scan_config(&self) {
        if let Some(path) = Self::get_scan_config_path() {
            if let Ok(file) = std::fs::File::create(path) {
                #[derive(serde::Serialize)]
                struct ScanConfig<'a> {
                    exclusions: &'a [PathBuf],
                    inclusions: &'a [PathBuf],
                }
                let config = ScanConfig {
                    exclusions: &self.scan_exclusions,
                    inclusions: &self.scan_inclusions,
                };
                let writer = std::io::BufWriter::new(file);
                let _ = serde_json::to_writer_pretty(writer, &config);
            }
        }
    }

    fn get_app_config_path() -> Option<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "startux", "x-adox")?;
        let config_dir = proj_dirs.config_dir();
        if !config_dir.exists() {
            let _ = std::fs::create_dir_all(config_dir);
        }
        Some(config_dir.join("app_config.json"))
    }

    fn load_app_config() -> Option<PathBuf> {
        let path = Self::get_app_config_path()?;
        let file = std::fs::File::open(path).ok()?;
        let reader = std::io::BufReader::new(file);

        #[derive(serde::Deserialize)]
        struct AppConfig {
            selected_xplane_path: Option<PathBuf>,
        }

        let config: AppConfig = serde_json::from_reader(reader).ok()?;
        config.selected_xplane_path
    }

    fn save_app_config(&self) {
        if let Some(path) = Self::get_app_config_path() {
            if let Ok(file) = std::fs::File::create(path) {
                #[derive(serde::Serialize)]
                struct AppConfig<'a> {
                    selected_xplane_path: Option<&'a PathBuf>,
                }
                let config = AppConfig {
                    selected_xplane_path: self.xplane_root.as_ref(),
                };
                let writer = std::io::BufWriter::new(file);
                let _ = serde_json::to_writer_pretty(writer, &config);
            }
        }
    }
}

fn is_path_excluded(path: &Path, exclusions: &[PathBuf]) -> bool {
    let p = path.to_string_lossy().to_string();
    let p_clean = p.trim_end_matches('/').to_string();

    exclusions.iter().any(|ex| {
        let e = ex.to_string_lossy().to_string();
        let e_clean = e.trim_end_matches('/').to_string();

        p_clean == e_clean || p_clean.starts_with(&(e_clean.clone() + "/"))
    })
}

// Data loading functions
fn load_packs(root: Option<PathBuf>) -> Result<Arc<Vec<SceneryPack>>, String> {
    let root = root.ok_or("X-Plane root not found")?;
    let xpm = XPlaneManager::new(&root).map_err(|e| e.to_string())?;
    let mut sm = SceneryManager::new(xpm.get_scenery_packs_path());
    sm.load().map_err(|e| e.to_string())?;
    Ok(Arc::new(sm.packs))
}

fn toggle_plugin(root: Option<PathBuf>, path: PathBuf, enable: bool) -> Result<(), String> {
    let root = root.ok_or("X-Plane root not found")?;
    ModManager::set_plugin_enabled(&root, &path, enable).map_err(|e| e.to_string())?;
    Ok(())
}

fn load_aircraft(
    root: Option<PathBuf>,
    exclusions: Vec<PathBuf>,
) -> Result<Arc<Vec<DiscoveredAddon>>, String> {
    let root = root.ok_or("X-Plane root not found")?;
    // Canonicalize root for robust exclusion matching
    let root = root.canonicalize().unwrap_or(root);

    let mut cache = x_adox_core::cache::DiscoveryCache::load();
    let aircraft = DiscoveryManager::scan_aircraft(&root, &mut cache, &exclusions);
    let _ = cache.save();

    Ok(Arc::new(aircraft))
}

fn load_plugins(root: Option<PathBuf>) -> Result<Arc<Vec<DiscoveredAddon>>, String> {
    let root = root.ok_or("X-Plane root not found")?;
    let mut cache = x_adox_core::cache::DiscoveryCache::load();
    let plugins = DiscoveryManager::scan_plugins(&root, &mut cache);
    let _ = cache.save();
    Ok(Arc::new(plugins))
}

fn load_csls(root: Option<PathBuf>) -> Result<Arc<Vec<DiscoveredAddon>>, String> {
    let root = root.ok_or("X-Plane root not found")?;
    let mut cache = x_adox_core::cache::DiscoveryCache::load();
    let csls = DiscoveryManager::scan_csls(&root, &mut cache);
    let _ = cache.save();
    Ok(Arc::new(csls))
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

fn load_aircraft_tree(
    root: Option<PathBuf>,
    exclusions: Vec<PathBuf>,
) -> Result<Arc<AircraftNode>, String> {
    let root = root.ok_or("X-Plane root not found")?;
    // Canonicalize root for robust exclusion matching
    let root = root.canonicalize().unwrap_or(root);

    let aircraft_path = root.join("Aircraft");
    let disabled_path = root.join("Aircraft (Disabled)");

    if !aircraft_path.exists() {
        return Err("Aircraft folder not found".to_string());
    }

    // Load BitNet model for tagging
    let bitnet = BitNetModel::new().unwrap_or_default();

    let merged_node = build_merged_aircraft_tree(
        &aircraft_path,
        &disabled_path,
        Path::new(""),
        &bitnet,
        &exclusions,
    );

    Ok(Arc::new(merged_node))
}

fn build_merged_aircraft_tree(
    enabled_root: &Path,
    disabled_root: &Path,
    relative_path: &Path,
    bitnet: &BitNetModel,
    exclusions: &[PathBuf],
) -> AircraftNode {
    let enabled_full = enabled_root.join(relative_path);
    let disabled_full = disabled_root.join(relative_path);

    let name = relative_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Addon Library".to_string());

    let mut children = Vec::new();
    let mut acf_file = None;
    let mut is_enabled = true;

    // Use absolute path for identifying which root it's in actually is_enabled
    // But we are merging. An aircraft folder (containing .acf) will exist in ONLY ONE of them.
    // So we check which one contains .acf.

    let mut entries_map = std::collections::BTreeSet::new();
    if let Ok(entries) = std::fs::read_dir(&enabled_full) {
        for entry in entries.flatten() {
            entries_map.insert(entry.file_name());
        }
    }
    if let Ok(entries) = std::fs::read_dir(&disabled_full) {
        for entry in entries.flatten() {
            entries_map.insert(entry.file_name());
        }
    }

    for entry_name in entries_map {
        let entry_name_str = entry_name.to_string_lossy().to_string();
        if entry_name_str.starts_with('.') {
            continue;
        }

        let entry_rel = relative_path.join(&entry_name);
        let e_full = enabled_root.join(&entry_rel);
        let d_full = disabled_root.join(&entry_rel);

        if e_full.is_dir() || d_full.is_dir() {
            // Check if this folder itself is excluded
            if is_path_excluded(&e_full, exclusions) || is_path_excluded(&d_full, exclusions) {
                continue;
            }

            children.push(build_merged_aircraft_tree(
                enabled_root,
                disabled_root,
                &entry_rel,
                bitnet,
                exclusions,
            ));
        } else if entry_name_str.ends_with(".acf") {
            acf_file = Some(entry_name_str);
            // If the .acf is found in disabled_full, then this whole folder is disabled.
            if disabled_full.join(&entry_name).exists() {
                is_enabled = false;
            }
        }
    }

    // Determine path for display/actions
    // If it's disabled, we MUST use the d_full path for actions to work.
    let final_path = if !is_enabled {
        disabled_full
    } else {
        enabled_full
    };

    // Predict tags
    let tags = if acf_file.is_some()
        || (!children.is_empty() && children.iter().any(|c| c.acf_file.is_some()))
    {
        bitnet.predict_aircraft_tags(&name, &final_path)
    } else {
        Vec::new()
    };

    AircraftNode {
        name,
        path: final_path,
        is_folder: acf_file.is_none() && !children.is_empty(),
        is_expanded: relative_path.as_os_str().is_empty(), // Expand root
        children,
        acf_file,
        is_enabled,
        tags,
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

async fn install_addon(
    root: Option<PathBuf>,
    zip_path: PathBuf,
    tab: Tab,
    dest_override: Option<PathBuf>,
    on_progress: impl FnMut(f32) + Send + 'static,
) -> Result<String, String> {
    let res = tokio::task::spawn_blocking(move || {
        extract_zip_task(root, zip_path, tab, dest_override, on_progress)
    })
    .await
    .map_err(|e| e.to_string())?;
    res
}

fn toggle_aircraft(root: Option<PathBuf>, path: PathBuf, enable: bool) -> Result<(), String> {
    let root = root.ok_or("X-Plane root not found")?;
    ModManager::set_aircraft_enabled(&root, &path, enable).map_err(|e| e.to_string())?;
    Ok(())
}

fn extract_zip_task(
    root: Option<PathBuf>,
    zip_path: PathBuf,
    tab: Tab,
    dest_override: Option<PathBuf>,
    mut on_progress: impl FnMut(f32) + Send + 'static,
) -> Result<String, String> {
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

    let root = root.ok_or("X-Plane root not found".to_string())?;

    let dest_dir = if let Some(dest) = dest_override {
        dest
    } else {
        match tab {
            Tab::Aircraft => root.join("Aircraft"),
            Tab::Plugins => root.join("Resources").join("plugins"),
            Tab::Scenery => root.join("Custom Scenery"),
            Tab::CSLs => root.join("Resources").join("plugins"),
            _ => return Err("Unsupported install tab".to_string()),
        }
    };

    // Extract to destination
    let total_files = archive.len();
    for i in 0..total_files {
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
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to copy file content: {}", e))?;
        }

        // Send progress
        on_progress((i as f32 / total_files as f32) * 100.0);
    }

    // Special handling for Scenery: add to scenery_packs.ini
    if matches!(tab, Tab::Scenery) {
        let xpm = XPlaneManager::new(&root).map_err(|e| e.to_string())?;
        let mut sm = SceneryManager::new(xpm.get_scenery_packs_path());
        sm.load().map_err(|e| e.to_string())?;
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
            Tab::CSLs | Tab::Heuristics | Tab::Issues | Tab::Settings | Tab::Utilities => {
                unreachable!()
            }
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

async fn load_logbook_data(
    root: Option<PathBuf>,
) -> Result<Vec<x_adox_core::logbook::LogbookEntry>, String> {
    let root = root.ok_or("X-Plane root not found")?;
    let path = root
        .join("Output")
        .join("logbooks")
        .join("X-Plane Pilot.txt");

    if !path.exists() {
        return Err("X-Plane Pilot.txt not found in Output/logbooks".to_string());
    }

    x_adox_core::logbook::LogbookParser::parse_file(path).map_err(|e| e.to_string())
}

async fn load_airports_data(
    root: Option<PathBuf>,
) -> Result<Arc<std::collections::HashMap<String, x_adox_core::apt_dat::Airport>>, String> {
    let root = root.ok_or("X-Plane root not found")?;
    let xpm = XPlaneManager::new(&root).map_err(|e| e.to_string())?;
    let path = xpm.get_default_apt_dat_path();

    if !path.exists() {
        return Err("apt.dat not found in default scenery".to_string());
    }

    let airports =
        x_adox_core::apt_dat::AptDatParser::parse_file(&path).map_err(|e| e.to_string())?;

    let mut map = std::collections::HashMap::new();
    for apt in airports {
        map.insert(apt.id.clone(), apt);
    }
    Ok(Arc::new(map))
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

fn load_log_issues(root: Option<PathBuf>) -> Result<Arc<Vec<x_adox_core::LogIssue>>, String> {
    let root = root.ok_or("X-Plane root not found")?;
    let xpm = XPlaneManager::new(&root).map_err(|e| e.to_string())?;
    let issues = xpm.check_log().map_err(|e| e.to_string())?;
    Ok(Arc::new(issues))
}

fn simulate_sort_task(
    root: Option<PathBuf>,
    model: BitNetModel,
    context: x_adox_bitnet::PredictContext,
) -> Result<
    (
        Arc<Vec<SceneryPack>>,
        x_adox_core::scenery::validator::ValidationReport,
    ),
    String,
> {
    let root = root.ok_or("X-Plane root not found")?;
    let xpm = XPlaneManager::new(&root).map_err(|e| e.to_string())?;
    let mut sm = SceneryManager::new(xpm.get_scenery_packs_path());
    sm.load().map_err(|e| e.to_string())?;
    let (packs, report) = sm.simulate_sort(&model, &context);
    Ok((Arc::new(packs), report))
}

fn save_packs_task(root: Option<PathBuf>, packs: Arc<Vec<SceneryPack>>) -> Result<(), String> {
    let root = root.ok_or("X-Plane root not found")?;
    let xpm = XPlaneManager::new(&root).map_err(|e| e.to_string())?;
    let ini_path = xpm.get_scenery_packs_path();

    // Backups are now handled automatically by sm.save() in x-adox-core

    let mut sm = SceneryManager::new(ini_path);
    sm.packs = packs.as_ref().clone();
    sm.save().map_err(|e| e.to_string())
}

async fn apply_profile_task(root: Option<PathBuf>, profile: Profile) -> Result<(), String> {
    let root = root.ok_or("X-Plane root not found")?;

    // 1. Scenery
    ModManager::set_bulk_scenery_enabled(&root, &profile.scenery_states)
        .map_err(|e| e.to_string())?;

    // 2. Plugins & Aircraft require discovering current paths to correctly toggle
    let mut cache = x_adox_core::cache::DiscoveryCache::load();

    // Plugins
    let current_plugins = DiscoveryManager::scan_plugins(&root, &mut cache);
    for plugin in current_plugins {
        if let Some(&should_be_enabled) = profile
            .plugin_states
            .get(&plugin.path.to_string_lossy().to_string())
        {
            if plugin.is_enabled != should_be_enabled {
                ModManager::set_plugin_enabled(&root, &plugin.path, should_be_enabled)
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    // Aircraft
    let current_aircraft = DiscoveryManager::scan_aircraft(&root, &mut cache, &[]);
    for aircraft in current_aircraft {
        if let Some(&should_be_enabled) = profile
            .aircraft_states
            .get(&aircraft.path.to_string_lossy().to_string())
        {
            if aircraft.is_enabled != should_be_enabled {
                ModManager::set_aircraft_enabled(&root, &aircraft.path, should_be_enabled)
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(())
}
