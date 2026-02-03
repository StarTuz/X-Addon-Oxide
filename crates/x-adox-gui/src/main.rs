use std::process::{Command, Stdio};
// SPDX-License-Identifier: MIT
// Copyright (c) 2020 Austin Goudge
// Copyright (c) 2026 StarTuz

use iced::widget::{
    button, checkbox, column, container, horizontal_space, image, mouse_area, pick_list,
    progress_bar, responsive, row, scrollable, slider, stack, svg, text, text_editor, text_input,
    tooltip, Column, Row,
};
use iced::window::icon;
use iced::{
    Background, Border, Color, Element, Length, Padding, Point, Renderer, Shadow, Subscription,
    Task, Theme,
};
use iced::keyboard;
use iced::mouse;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use x_adox_bitnet::BitNetModel;
use x_adox_core::discovery::{AddonType, DiscoveredAddon, DiscoveryManager};
use x_adox_core::management::{AddonType as ManagementAddonType, ModManager};
use x_adox_core::profiles::{Profile, ProfileCollection, ProfileManager};
use x_adox_core::scenery::{SceneryManager, SceneryPack, SceneryPackType};
use x_adox_core::XPlaneManager;

mod map;
mod style;
use map::{MapView, TileManager};
use simplelog::*;
use std::fs::File;

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
    // Initialize logging
    let config_root = x_adox_core::get_config_root();
    let log_path = config_root.join("x-adox.log");
    
    if let Ok(log_file) = File::create(&log_path) {
        let _ = WriteLogger::init(
            LevelFilter::Debug,
            Config::default(),
            log_file,
        );
        log::info!("Logging initialized at {:?}", log_path);
    }

    let icon_data = include_bytes!("../../../icon.png");
    let window_icon = icon::from_file_data(icon_data, None).ok();

    iced::application("X-Addon-Oxide", App::update, App::view)
        .theme(|_| Theme::Dark)
        .subscription(App::subscription)
        .window(iced::window::Settings {
            size: [1100.0, 900.0].into(),
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
struct DragContext {
    source_index: usize,
    source_name: String,
    hover_target_index: Option<usize>,
    cursor_position: Point,
    is_over_basket: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeEdge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmType {
    DeleteAddon(Tab, String, PathBuf),
    BulkDeleteLogbook,
    BulkDeleteLogIssues,
}

#[derive(Debug, Clone)]
pub struct ModalState {
    pub title: String,
    pub message: String,
    pub confirm_type: ConfirmType,
    pub is_danger: bool,
}

#[derive(Debug, Clone)]
enum Message {
    // Tab navigation
    SwitchTab(Tab),

    // Scenery
    SceneryLoaded(Result<Arc<Vec<SceneryPack>>, String>),
    WindowResized(iced::Size),
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
    GotoScenery(String), // New message for navigation from reports
    HoverScenery(Option<String>),
    HoverAirport(Option<String>),
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
    DeleteAddonDirect(PathBuf, Tab),
    ConfirmDelete(Tab, bool),

    // Expansion & Scripts
    MapZoom {
        new_center: (f64, f64),
        new_zoom: f64,
    },
    FocusMap(f64, f64, f64), // (lat, lon, zoom)
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
    ClearOverrides,
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
    ExportLogIssues,

    // Sticky Sort (Phase 3)
    OpenPriorityEditor(String),
    UpdatePriorityValue(String, u8),
    SetPriority(String, u8),
    RemovePriority(String),
    CancelPriorityEdit,

    // Interactive Sorting (Phase 4)
    MovePack(String, MoveDirection),
    EnableAllScenery,
    ClearAllPins,

    // Drag and Drop (Phase 5)
    DragStart { index: usize, name: String },
    DragMove(Point),
    DragHover(usize),
    DragLeaveHover,
    DragEnterBasket,
    DragLeaveBasket,
    DragEnd,
    DragCancel,

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
    NewTagChanged(String),
    AddTag(String, String), // (PackName, Tag)
    RemoveTag(String, String), // (PackName, Tag)
    TagOperationComplete,

    // Launch X-Plane
    LaunchXPlane,
    LaunchArgsChanged(String),
    OpenLaunchHelp,
    CloseLaunchHelp,
    // Utilities
    LogbookLoaded(Result<Vec<x_adox_core::logbook::LogbookEntry>, String>),
    SelectFlight(Option<usize>),
    AirportsLoaded(
        Result<Arc<std::collections::HashMap<String, x_adox_core::apt_dat::Airport>>, String>,
    ),
    ToggleLogbook,
    LogbooksFound(Result<Vec<LogbookPath>, String>),
    SelectLogbook(LogbookPath),
    // Companion Apps
    LaunchCompanionApp,
    SelectCompanionApp(usize),
    ToggleCompanionManager,
    UpdateCompanionNameInput(String),
    BrowseForCompanionPath,
    CompanionPathSelected(Option<PathBuf>),
    AddCompanionApp,
    // Utilities - Companion Apps
    ToggleCompanionAutoLaunch(usize),
    StatusChanged(String),
    ToggleLogIssue(usize, bool),
    ToggleAllLogIssues(bool),
    RemoveCompanionApp(usize),
    ToggleMapFilterSettings,
    ToggleMapFilter(MapFilterType),

    // Logbook Filters
    LogbookFilterAircraftChanged(String),
    LogbookFilterCircularToggled(bool),
    LogbookFilterDurationMinChanged(String),
    LogbookFilterDurationMaxChanged(String),
    DeleteLogbookEntry(usize),
    ToggleLogbookEntrySelection(usize),
    ToggleAllLogbookSelection(bool),
    RequestLogbookBulkDelete,
    ConfirmLogbookBulkDelete,
    CancelLogbookBulkDelete,
    Tick(std::time::Instant),

    // Scenery Search
    ScenerySearchChanged(String),
    ScenerySearchNext,
    ScenerySearchPrev,
    ScenerySearchSubmit,
    ToggleBucketItem(String),
    ClearBucket,
    ToggleBucket,
    ToggleBasketSelection(String),
    ToggleAutopin(bool),
    DragBucketStart(Option<String>),
    DropBucketAt(usize),
    BasketDragStart,
    BasketDragged(Point),
    BasketDragEnd,
    BasketResizeStart(ResizeEdge),
    BasketResized(Point),
    BasketResizeEnd,
    ModifiersChanged(keyboard::Modifiers),

    // Backup & Restore
    BackupUserData,
    RestoreUserData,
    BackupComplete(Result<PathBuf, String>),
    RestoreComplete(Result<String, String>),

    // Modal
    ShowModal(ModalState),
    CloseModal,
    ConfirmModal(ConfirmType),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MapFilterType {
    CustomAirports,
    Enhancements,
    GlobalAirports,
    OrthoCoverage,
    OrthoMarkers,
    RegionalOverlays,
    FlightPaths,
    HealthScores,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogbookPath(pub PathBuf);

impl std::fmt::Display for LogbookPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        )
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CompanionApp {
    pub name: String,
    pub path: std::path::PathBuf,
    #[serde(default)]
    pub auto_launch: bool,
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
    pub fn progress(&self) -> f32 {
        let fields = [
            self.scenery,
            self.aircraft,
            self.aircraft_tree,
            self.plugins,
            self.csls,
            self.log_issues,
            self.airports,
            self.logbook,
        ];
        let completed = fields.iter().filter(|&&f| f).count();
        completed as f32 / fields.len() as f32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MapFilters {
    #[serde(default)]
    pub show_custom_airports: bool,
    #[serde(default)]
    pub show_enhancements: bool,
    #[serde(default)]
    pub show_global_airports: bool,
    #[serde(default)]
    pub show_ortho_coverage: bool,
    #[serde(default)]
    pub show_ortho_markers: bool,
    #[serde(default)]
    pub show_regional_overlays: bool,
    #[serde(default)]
    pub show_flight_paths: bool,
    #[serde(default)]
    pub show_health_scores: bool,
}

impl Default for MapFilters {
    fn default() -> Self {
        Self {
            show_custom_airports: true,
            show_enhancements: true,
            show_global_airports: true,
            show_ortho_coverage: false,
            show_ortho_markers: false,
            show_regional_overlays: false,
            show_flight_paths: true,
            show_health_scores: true,
        }
    }
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
    icon_grip: svg::Handle,
    // Map state
    hovered_scenery: Option<String>,
    hovered_airport_id: Option<String>,
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
    available_logbooks: Vec<LogbookPath>,
    selected_logbook_path: Option<LogbookPath>,

    // Pro Mode
    validation_report: Option<x_adox_core::scenery::validator::ValidationReport>,
    simulated_packs: Option<Arc<Vec<SceneryPack>>>,
    region_focus: Option<String>,

    animation_time: f32, // Pulsing/Animation state

    // Smart View Cache
    smart_groups: std::collections::BTreeMap<String, Vec<AircraftNode>>,
    smart_model_groups:
        std::collections::BTreeMap<String, std::collections::BTreeMap<String, Vec<AircraftNode>>>,

    // UI State for polish
    ignored_issues: std::collections::HashSet<(String, String)>, // (type, pack_name)
    expanded_issue_groups: std::collections::HashSet<String>,    // type
    loading_state: LoadingState,

    // Floating UI
    editing_priority: Option<(String, u8)>,
    drag_context: Option<DragContext>,
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
    show_launch_help: bool,

    // Phase 3
    new_tag_input: String,

    // Launch X-Plane
    launch_args: String,

    // Utilities - Companion Apps
    companion_apps: Vec<CompanionApp>,
    selected_companion_app: Option<usize>,
    show_companion_manage: bool,
    new_companion_name: String,
    new_companion_path: Option<PathBuf>,
    show_map_filter_settings: bool,
    map_filters: MapFilters,

    // Logbook Filtering
    logbook_filter_aircraft: String,
    logbook_filter_circular: bool,
    logbook_filter_duration_min: String,
    logbook_filter_duration_max: String,
    logbook_selection: std::collections::HashSet<usize>,
    show_logbook_bulk_delete_confirm: bool,
    selected_log_issues: std::collections::HashSet<usize>,
    last_scenery_click: Option<(String, std::time::Instant)>,

    // Scenery Search
    scenery_search_query: String,
    scenery_search_matches: Vec<usize>,
    scenery_search_index: Option<usize>,
    pub scenery_bucket: Vec<String>,
    pub scenery_last_bucket_index: Option<usize>,
    pub keyboard_modifiers: keyboard::Modifiers,
    icon_basket: svg::Handle,
    icon_paste: svg::Handle,
    pub show_scenery_basket: bool,
    pub selected_basket_items: std::collections::HashSet<String>,
    pub basket_offset: iced::Vector,
    pub basket_drag_origin: Option<Point>,
    pub is_basket_dragging: bool,
    pub basket_size: iced::Vector,
    pub active_resize_edge: Option<ResizeEdge>,
    pub window_size: iced::Size,
    pub autopin_enabled: bool,
    pub scenery_is_saving: bool,
    pub scenery_save_pending: bool,

    // Modal
    active_modal: Option<ModalState>,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        // Discover all X-Plane installations
        let available_roots = XPlaneManager::find_all_xplane_roots();

        // Try to load persisted selection, fallback to first available or try_find_root
        let (saved_root, companion_apps, map_filters) = Self::load_app_config();
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
            hovered_airport_id: None,
            map_zoom: 0.0,
            map_center: (0.0, 0.0),
            map_initialized: false,
            scenery_scroll_id: scrollable::Id::unique(),
            install_progress: None,
            // Heuristics are GLOBAL (not per-install) - use BitNetModel's global config path
            heuristics_model: root.as_ref()
                .map(|r| Self::initialize_heuristics(r))
                .unwrap_or_else(|| BitNetModel::new().unwrap_or_default()),
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
            drag_context: None,
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
            icon_grip: svg::Handle::from_memory(
                include_bytes!("../assets/icons/grab_hand.svg").to_vec(),
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
            animation_time: 0.0,
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
            show_launch_help: false,

            // Phase 3
            new_tag_input: String::new(),
            validation_report: None,

            // Utilities
            logbook: Vec::new(),
            selected_flight: None,
            airports: Arc::new(std::collections::HashMap::new()),
            logbook_expanded: false,
            available_logbooks: Vec::new(),
            selected_logbook_path: None,

            // Launch X-Plane
            launch_args: String::new(),

            // Companion Apps
            companion_apps,
            selected_companion_app: None,
            show_companion_manage: false,
            new_companion_name: String::new(),
            new_companion_path: None,
            show_map_filter_settings: false,
            map_filters,
            logbook_filter_aircraft: String::new(),
            logbook_filter_circular: false,
            logbook_filter_duration_min: String::new(),
            logbook_filter_duration_max: String::new(),
            logbook_selection: std::collections::HashSet::new(),
            show_logbook_bulk_delete_confirm: false,
            selected_log_issues: std::collections::HashSet::new(),
            smart_groups: std::collections::BTreeMap::new(),
            smart_model_groups: std::collections::BTreeMap::new(),
            last_scenery_click: None,

            // Scenery Search
            scenery_search_query: String::new(),
            scenery_search_matches: Vec::new(),
            scenery_search_index: None,
            scenery_bucket: Vec::new(),
            scenery_last_bucket_index: None,
            keyboard_modifiers: keyboard::Modifiers::default(),
            icon_basket: svg::Handle::from_memory(
                include_bytes!("../assets/icons/basket.svg").to_vec(),
            ),
            icon_paste: svg::Handle::from_memory(
                include_bytes!("../assets/icons/paste.svg").to_vec(),
            ),
            show_scenery_basket: false,
            selected_basket_items: std::collections::HashSet::new(),
            basket_offset: iced::Vector::new(20.0, 60.0), // Margin from Right (x) and Top (y)
            basket_drag_origin: None,
            is_basket_dragging: false,
            basket_size: iced::Vector::new(350.0, 400.0),
            active_resize_edge: None,
            window_size: iced::Size::new(1280.0, 720.0),
            autopin_enabled: true, // Enabled by default as it's a "Smart" feature
            scenery_is_saving: false,
            scenery_save_pending: false,
            active_modal: None,
        };

        if let Some(pm) = &app.profile_manager {
            if let Ok(collection) = pm.load() {
                app.profiles = collection;
                
                // Apply active profile's pins on startup
                if let Some(active_name) = &app.profiles.active_profile {
                    if let Some(profile) = app.profiles.profiles.iter().find(|p| p.name == *active_name) {
                        let overrides = profile.scenery_overrides.iter()
                            .map(|(k, v)| (k.clone(), *v))
                            .collect::<std::collections::BTreeMap<_, _>>();
                        app.heuristics_model.apply_overrides(overrides);
                    }
                }

                // Initial sync to capture any existing heuristics.json overrides into the active profile
                app.sync_active_profile_scenery();
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
                Task::perform(
                    async move {
                        x_adox_core::logbook::LogbookParser::find_logbooks(r8)
                            .map(|paths| paths.into_iter().map(LogbookPath).collect::<Vec<_>>())
                            .map_err(|e| e.to_string())
                    },
                    Message::LogbooksFound,
                ),
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

    fn sync_active_profile_scenery(&mut self) {
        let states = self.packs
            .iter()
            .map(|p| (p.name.clone(), p.status == SceneryPackType::Active))
            .collect();
        self.profiles.update_active_scenery(states);

        // Also sync manual reorder pins (scenery overrides) into the profile
        let overrides = self.heuristics_model.config.overrides.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        self.profiles.update_active_overrides(overrides);
        
        self.save_profiles();
        // [DEBUG]
        log::debug!("Synced {} overrides to active profile", self.profiles.get_active_profile_mut().map(|p| p.scenery_overrides.len()).unwrap_or(0));
    }

    fn count_disabled_scenery(&self) -> usize {
        self.packs
            .iter()
            .filter(|p| p.status == SceneryPackType::Disabled)
            .count()
    }

    fn trigger_scenery_save(&mut self) -> Task<Message> {
        if self.scenery_is_saving {
            self.scenery_save_pending = true;
            return Task::none();
        }

        if let Some(root) = self.xplane_root.clone() {
            self.scenery_is_saving = true;
            self.scenery_save_pending = false;
            let current_packs = (*self.packs).clone();
            let model = self.heuristics_model.clone();
            return Task::perform(
                async move { save_scenery_packs(root, current_packs, model) },
                Message::PackToggled,
            );
        }
        Task::none()
    }

    fn sync_active_profile_plugins(&mut self) {
        let mut states: std::collections::HashMap<String, bool> = self.plugins
            .iter()
            .map(|p| (p.path.to_string_lossy().to_string(), p.is_enabled))
            .collect();
        // Also merge CSLs into plugin_states as they use the same management logic
        for csl in &*self.csls {
            states.insert(csl.path.to_string_lossy().to_string(), csl.is_enabled);
        }
        self.profiles.update_active_plugins(states);
        self.save_profiles();
    }

    fn sync_active_profile_aircraft(&mut self) {
        let states = self.aircraft
            .iter()
            .map(|p| (p.path.to_string_lossy().to_string(), p.is_enabled))
            .collect();
        self.profiles.update_active_aircraft(states);
        self.save_profiles();
    }

    fn save_profiles(&self) {
        if let Some(ref pm) = self.profile_manager {
            let _ = pm.save(&self.profiles);
        }
    }

    fn scroll_to_scenery_index(&self, index: usize) -> Task<Message> {
        // Height: 75px card + 2px spacing (from view_scenery column spacing)
        let card_height = 75.0; 
        let spacing = 2.0;
        let offset = index as f32 * (card_height + spacing);

        scrollable::scroll_to(
            self.scenery_scroll_id.clone(),
            scrollable::AbsoluteOffset { x: 0.0, y: offset },
        )
    }

    /// Merges airports discovered in scenery packs into the global airports map.
    /// Custom pack airports take precedence over global airports if they have valid coordinates.
    fn merge_custom_airports(&mut self) {
        let mut merged = (*self.airports).clone();
        let mut added = 0usize;

        for pack in self.packs.iter() {
            for airport in &pack.airports {
                // Only merge if the airport has valid coordinates
                if airport.lat.is_some() && airport.lon.is_some() {
                    // Insert or replace (custom packs take precedence)
                    if !merged.contains_key(&airport.id) {
                        added += 1;
                    }
                    merged.insert(airport.id.clone(), airport.clone());
                }
            }
        }

        if added > 0 {
            println!("[App] Merged {} custom airports into map database", added);
        }
        self.airports = Arc::new(merged);
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
            Message::FocusMap(lat, lon, zoom) => {
                self.map_center = (lat, lon);
                self.map_zoom = zoom;
                self.map_initialized = true;
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
                    if let Some(log_path) = &self.selected_logbook_path {
                        return Task::perform(load_logbook_data(log_path.0.clone()), Message::LogbookLoaded);
                    }
                }

                Task::none()
            }
            Message::LogbooksFound(result) => {
                match result {
                    Ok(logbooks) => {
                        self.available_logbooks = logbooks;
                        if self.selected_logbook_path.is_none() {
                            // Default to "X-Plane Pilot.txt" if found
                            if let Some(pilot) = self.available_logbooks.iter().find(|p| {
                                p.0.file_name()
                                    .and_then(|s| s.to_str())
                                    .map(|s| s == "X-Plane Pilot.txt")
                                    .unwrap_or(false)
                            }) {
                                self.selected_logbook_path = Some(pilot.clone());
                            } else {
                                self.selected_logbook_path = self.available_logbooks.first().cloned();
                            }
                        }

                        if let Some(log_path) = &self.selected_logbook_path {
                            return Task::perform(
                                load_logbook_data(log_path.0.clone()),
                                Message::LogbookLoaded,
                            );
                        } else {
                            self.loading_state.logbook = true;
                            self.check_loading_complete();
                        }
                    }
                    Err(e) => {
                        self.status = format!("Failed to find logbooks: {}", e);
                        self.loading_state.logbook = true;
                        self.check_loading_complete();
                    }
                }
                Task::none()
            }
            Message::SelectLogbook(log_path) => {
                self.selected_logbook_path = Some(log_path.clone());
                Task::perform(load_logbook_data(log_path.0), Message::LogbookLoaded)
            }
            Message::LogIssuesLoaded(result) => {
                self.loading_state.log_issues = true;
                match result {
                    Ok(issues) => {
                        self.log_issues = issues;
                        // Select all by default
                        self.selected_log_issues = (0..self.log_issues.len()).collect();
                        if !self.log_issues.is_empty() && !self.loading_state.is_loading {
                            self.status =
                                format!("Found {} issues in Log.txt", self.log_issues.len());
                        }
                    }
                    Err(e) => {
                        self.status = format!("Failed to read Log.txt: {}", e);
                    }
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
            Message::ExportLogIssues => {
                let issues = self.log_issues.clone();
                let selection = self.selected_log_issues.clone();
                
                if selection.is_empty() {
                    self.status = "No issues selected for export".to_string();
                    return Task::none();
                }

                Task::perform(
                    async move { 
                        // Filter issues based on selection
                        let filtered: Vec<_> = issues.iter().enumerate()
                            .filter(|(i, _)| selection.contains(i))
                            .map(|(_, issue)| issue.clone())
                            .collect();
                        export_log_issues_task(Arc::new(filtered)) 
                    },
                    |result| match result {
                        Ok(path) => Message::StatusChanged(format!("Report exported to {}", path.display())),
                        Err(e) => Message::StatusChanged(format!("Export failed: {}", e)),
                    },
                )
            }
            Message::SceneryLoaded(result) => {
                self.loading_state.scenery = true;
                match result {
                    Ok(packs) => {
                        self.packs = packs;

                        // Merge custom pack airports into the global database for map lookups
                        self.merge_custom_airports();

                        // Auto-run validation whenever scenery is loaded
                        self.validation_report = Some(
                            x_adox_core::scenery::validator::SceneryValidator::validate(&self.packs),
                        );

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
                        self.update_smart_view_cache();
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

                        // Merge custom pack airports if scenery is already loaded
                        if self.loading_state.scenery {
                            self.merge_custom_airports();
                        }

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
                if self.selected_flight == index {
                    self.selected_flight = None;
                } else {
                    self.selected_flight = index;
                }
                Task::none()
            }
            Message::ToggleLogbook => {
                self.logbook_expanded = !self.logbook_expanded;
                Task::none()
            }
            Message::LogbookFilterAircraftChanged(val) => {
                self.logbook_filter_aircraft = val;
                Task::none()
            }
            Message::LogbookFilterCircularToggled(val) => {
                self.logbook_filter_circular = val;
                Task::none()
            }
            Message::LogbookFilterDurationMinChanged(val) => {
                self.logbook_filter_duration_min = val;
                Task::none()
            }
            Message::LogbookFilterDurationMaxChanged(val) => {
                self.logbook_filter_duration_max = val;
                Task::none()
            }
            Message::DeleteLogbookEntry(idx) => {
                if let Some(root) = &self.xplane_root {
                    let path = root
                        .join("Output")
                        .join("logbooks")
                        .join("X-Plane Pilot.txt");
                    if idx < self.logbook.len() {
                        self.logbook.remove(idx);
                        // Clear selection when individual deletion happens to avoid index mismatch
                        self.logbook_selection.clear();
                        let entries = self.logbook.clone();
                        return Task::perform(
                            async move {
                                x_adox_core::logbook::LogbookParser::save_file(path, &entries)
                                    .map_err(|e| e.to_string())
                            },
                            |res| match res {
                                Ok(_) => Message::Refresh,
                                Err(_e) => {
                                    // In a real app we'd dispatch a SetStatus message
                                    Message::Refresh
                                }
                            },
                        );
                    }
                }
                Task::none()
            }
            Message::ToggleLogbookEntrySelection(idx) => {
                if self.logbook_selection.contains(&idx) {
                    self.logbook_selection.remove(&idx);
                } else {
                    self.logbook_selection.insert(idx);
                }
                Task::none()
            }
            Message::ToggleAllLogbookSelection(select) => {
                let filtered_indices = self.filtered_logbook_indices();
                if select {
                    for idx in filtered_indices {
                        self.logbook_selection.insert(idx);
                    }
                } else {
                    for idx in filtered_indices {
                        self.logbook_selection.remove(&idx);
                    }
                }
                Task::none()
            }
            Message::RequestLogbookBulkDelete => {
                self.show_logbook_bulk_delete_confirm = true;
                Task::none()
            }
            Message::CancelLogbookBulkDelete => {
                self.show_logbook_bulk_delete_confirm = false;
                Task::none()
            }
            Message::ConfirmLogbookBulkDelete => {
                self.show_logbook_bulk_delete_confirm = false;
                if let Some(root) = &self.xplane_root {
                    let path = root
                        .join("Output")
                        .join("logbooks")
                        .join("X-Plane Pilot.txt");

                    let mut indices: Vec<usize> = self.logbook_selection.iter().cloned().collect();
                    indices.sort_by(|a, b| b.cmp(a)); // Sort descending to maintain relative indices while removing

                    for idx in indices {
                        if idx < self.logbook.len() {
                            self.logbook.remove(idx);
                        }
                    }

                    self.logbook_selection.clear();
                    let entries = self.logbook.clone();
                    return Task::perform(
                        async move {
                            x_adox_core::logbook::LogbookParser::save_file(path, &entries)
                                .map_err(|e| e.to_string())
                        },
                        |res| match res {
                            Ok(_) => Message::Refresh,
                            Err(_e) => Message::Refresh,
                        },
                    );
                }
                Task::none()
            }
            Message::LaunchCompanionApp => {
                if let Some(idx) = self.selected_companion_app {
                    if let Some(app) = self.companion_apps.get(idx).cloned() {
                        self.spawn_companion_app(&app);
                        if self.status != format!("Failed to launch {}", app.name) {
                            self.status = format!("Launched {}", app.name);
                        }
                    }
                }
                Task::none()
            }
            Message::SelectCompanionApp(idx) => {
                self.selected_companion_app = Some(idx);
                Task::none()
            }
            Message::ToggleCompanionManager => {
                self.show_companion_manage = !self.show_companion_manage;
                Task::none()
            }
            Message::UpdateCompanionNameInput(name) => {
                self.new_companion_name = name;
                Task::none()
            }
            Message::ToggleCompanionAutoLaunch(idx) => {
                if let Some(app) = self.companion_apps.get_mut(idx) {
                    app.auto_launch = !app.auto_launch;
                    let _ = self.save_app_config();
                }
                Task::none()
            }
            Message::ToggleMapFilterSettings => {
                self.show_map_filter_settings = !self.show_map_filter_settings;
                Task::none()
            }
            Message::ToggleMapFilter(filter_type) => {
                match filter_type {
                    MapFilterType::CustomAirports => {
                        self.map_filters.show_custom_airports =
                            !self.map_filters.show_custom_airports;
                    }
                    MapFilterType::Enhancements => {
                        self.map_filters.show_enhancements = !self.map_filters.show_enhancements;
                    }
                    MapFilterType::GlobalAirports => {
                        self.map_filters.show_global_airports =
                            !self.map_filters.show_global_airports;
                    }
                    MapFilterType::OrthoCoverage => {
                        self.map_filters.show_ortho_coverage =
                            !self.map_filters.show_ortho_coverage;
                    }
                    MapFilterType::OrthoMarkers => {
                        self.map_filters.show_ortho_markers = !self.map_filters.show_ortho_markers;
                    }
                    MapFilterType::RegionalOverlays => {
                        self.map_filters.show_regional_overlays =
                            !self.map_filters.show_regional_overlays;
                    }
                    MapFilterType::FlightPaths => {
                        self.map_filters.show_flight_paths = !self.map_filters.show_flight_paths;
                    }
                    MapFilterType::HealthScores => {
                        self.map_filters.show_health_scores = !self.map_filters.show_health_scores;
                    }
                }
                self.save_app_config();
                Task::none()
            }
            Message::BrowseForCompanionPath => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title("Select Companion Application Executable")
                        .pick_file()
                        .await
                        .map(|f| f.path().to_path_buf())
                },
                Message::CompanionPathSelected,
            ),
            Message::CompanionPathSelected(path) => {
                if let Some(ref p) = path {
                    if self.new_companion_name.is_empty() {
                        if let Some(file_name) = p.file_stem() {
                            self.new_companion_name = file_name.to_string_lossy().to_string();
                        }
                    }
                }
                self.new_companion_path = path;
                Task::none()
            }
            Message::AddCompanionApp => {
                if let (Some(path), name) = (self.new_companion_path.clone(), self.new_companion_name.clone()) {
                    if !name.is_empty() {
                        let app = CompanionApp {
                            name,
                            path,
                            auto_launch: false,
                        };
                        self.companion_apps.push(app);
                        self.new_companion_name.clear();
                        self.new_companion_path = None;
                        self.save_app_config();
                        self.selected_companion_app = Some(self.companion_apps.len() - 1);
                    }
                }
                Task::none()
            }
            Message::RemoveCompanionApp(idx) => {
                if idx < self.companion_apps.len() {
                    self.companion_apps.remove(idx);
                    if self.selected_companion_app == Some(idx) {
                        self.selected_companion_app = None;
                    } else if let Some(s_idx) = self.selected_companion_app {
                        if s_idx > idx {
                            self.selected_companion_app = Some(s_idx - 1);
                        }
                    }
                    self.save_app_config();
                }
                Task::none()
            }
            Message::PluginsLoaded(result) => {
                self.loading_state.plugins = true;
                match result {
                    Ok(plugins) => {
                        self.plugins = plugins;
                        if !self.loading_state.is_loading {
                            self.sync_active_profile_plugins();
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
                            self.sync_active_profile_plugins(); // CSLs share plugin_states
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
                    log::debug!("Pre-syncing current state before switching to profile '{}'", name);
                    self.sync_active_profile_scenery();
                    self.sync_active_profile_plugins();
                    self.sync_active_profile_aircraft();

                    self.profiles.active_profile = Some(name.clone());
                    self.launch_args = profile.launch_args.clone(); // Load launch args from profile
                    let pm = self.profile_manager.clone();
                    let collection = self.profiles.clone();
                    let root = self.xplane_root.clone();

                    // Save active profile choice
                    if let Some(pm) = &pm {
                        let _ = pm.save(&collection);
                    }

                    // Apply profile-specific scenery overrides (Pins)
                    let new_overrides = profile.scenery_overrides
                        .iter()
                        .map(|(k, v)| (k.clone(), *v))
                        .collect::<std::collections::BTreeMap<_, _>>();
                    log::debug!("Switching Profile: Applying {} overrides from profile '{}'", new_overrides.len(), name);
                    self.heuristics_model.apply_overrides(new_overrides);

                    self.status = format!("Switching to profile {}...", name);
                    let model = self.heuristics_model.clone();
                    Task::perform(
                        async move { apply_profile_task(root, profile, model).await },
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
                let mut plugin_states: std::collections::HashMap<String, bool> = self
                    .plugins
                    .iter()
                    .map(|p| (p.path.to_string_lossy().to_string(), p.is_enabled))
                    .collect();
                for csl in &*self.csls {
                    plugin_states.insert(csl.path.to_string_lossy().to_string(), csl.is_enabled);
                }

                let scenery_overrides = self.heuristics_model.config.overrides
                    .iter()
                    .map(|(k, v)| (k.clone(), *v))
                    .collect();

                let new_profile = Profile {
                    name: name.clone(),
                    scenery_states,
                    aircraft_states,
                    plugin_states,
                    scenery_overrides,
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
                self.save_profiles();
                Task::done(Message::Refresh)
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

                                match Command::new(&exe)
                                    .args(&args_vec)
                                    .current_dir(root)
                                    .stdin(Stdio::null())
                                    .stdout(Stdio::null())
                                    .stderr(Stdio::null())
                                    // Escape AppImage sandbox if necessary
                                    .env_remove("LD_LIBRARY_PATH")
                                    .env_remove("APPDIR")
                                    .env_remove("APPIMAGE")
                                    .spawn()
                                {
                                    Ok(_) => {
                                        self.status = "X-Plane launched!".to_string();

                                        // Auto-launch companion apps
                                        for app in self.companion_apps.clone() {
                                            if app.auto_launch {
                                                self.spawn_companion_app(&app);
                                            }
                                        }
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
                self.profiles.update_active_launch_args(args);
                self.save_profiles();
                Task::none()
            }
            Message::TagOperationComplete => Task::none(),
            Message::OpenLaunchHelp => {
                self.show_launch_help = true;
                Task::none()
            }
            Message::CloseLaunchHelp => {
                self.show_launch_help = false;
                Task::none()
            }
            Message::NewTagChanged(txt) => {
                self.new_tag_input = txt;
                Task::none()
            }
            Message::AddTag(pack_name, tag) => {
                let tag = tag.trim().to_string();
                if !tag.is_empty() {
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
            Message::InstallCSL => Task::perform(pick_archive("CSL"), |p| {
                Message::InstallPicked(Tab::CSLs, p)
            }),
            Message::TogglePack(name) => {
                let mut new_packs = (*self.packs).clone();
                let mut toggled_status = None;

                if let Some(pack) = new_packs.iter_mut().find(|p| p.name == name) {
                    let enable = pack.status == SceneryPackType::Disabled;
                    pack.status = if enable {
                        SceneryPackType::Active
                    } else {
                        SceneryPackType::Disabled
                    };
                    toggled_status = Some(pack.status.clone());
                }

                if let Some(status) = toggled_status {
                    self.packs = Arc::new(new_packs);
                    self.sync_active_profile_scenery();

                    self.status = format!(
                        "{} {}...",
                        if status == SceneryPackType::Active {
                            "Enabling"
                        } else {
                            "Disabling"
                        },
                        name
                    );

                    return self.trigger_scenery_save();
                }
                Task::none()
            }
            Message::PackToggled(result) => {
                self.scenery_is_saving = false;
                match result {
                    Ok(()) => {
                        self.status = "Saved!".to_string();
                        if self.scenery_save_pending {
                            return self.trigger_scenery_save();
                        }
                        Task::none()
                    }
                    Err(e) => {
                        self.status = format!("Error saving scenery: {}", e);
                        self.scenery_save_pending = false; // Stop queue on error
                        Task::none()
                    }
                }
            }
            Message::StatusChanged(status) => {
                self.status = status;
                Task::none()
            }
            Message::ToggleLogIssue(idx, is_selected) => {
                if is_selected {
                    self.selected_log_issues.insert(idx);
                } else {
                    self.selected_log_issues.remove(&idx);
                }
                Task::none()
            }
            Message::ToggleAllLogIssues(select_all) => {
                if select_all {
                    self.selected_log_issues = (0..self.log_issues.len()).collect();
                } else {
                    self.selected_log_issues.clear();
                }
                Task::none()
            }
            Message::SmartSort => {
                let root = self.xplane_root.clone();
                let context = x_adox_bitnet::PredictContext {
                    region_focus: self.region_focus.clone(),
                    ..Default::default()
                };
                let model = self.heuristics_model.clone();
                let packs = self.packs.clone();
                self.status = "Simulating sort...".to_string();
                Task::perform(
                    async move { simulate_sort_task(root, model, context, packs) },
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
                // validation_report is NOT cleared here; it will be refreshed by the subsequent reload
                self.status = "Applying changes...".to_string();
                let model = self.heuristics_model.clone();
                Task::perform(
                    async move { save_packs_task(root, packs_to_save, model) },
                    Message::PackToggled,
                )
            }
            Message::CancelSort => {
                self.simulated_packs = None;
                // validation_report is NOT cleared here; preserves current state
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
                            // NOTE: This AutoFix has been disabled because it conflicts with custom sort rules.
                            // If the user has customized the SimHeaven score to be lower (higher priority),
                            // they WANT SimHeaven above Global Airports. The Smart Sort already handles
                            // the ordering based on the user's custom rules.
                            // The old code would forcibly move SimHeaven BELOW Global Airports,
                            // which is incorrect when the user has customized their sorting preferences.
                        }
                        "mesh_above_overlay" => {
                            // PERMANENT FIX: Add override rules for misclassified mesh/ortho packs
                            // This teaches the sorting engine to treat these packs as Mesh (Score 30)
                            let mut added_overrides = Vec::new();

                            if let Some(report) = &self.validation_report {
                                for issue in &report.issues {
                                    if issue.issue_type == "mesh_above_overlay"
                                        && !self.ignored_issues.contains(&(
                                            issue.issue_type.clone(),
                                            issue.pack_name.clone(),
                                        ))
                                    {
                                        // Add this pack to overrides with Mesh score (60 = bottom priority)
                                        let config = Arc::make_mut(&mut self.heuristics_model.config);
                                        if !config.overrides.contains_key(&issue.pack_name) {
                                            config.overrides.insert(issue.pack_name.clone(), 60);
                                            added_overrides.push(issue.pack_name.clone());
                                        }
                                    }
                                }
                            }

                            if !added_overrides.is_empty() {
                                // Save the updated heuristics
                                if let Err(e) = self.heuristics_model.save() {
                                    self.status = format!("AutoFix failed to save: {}", e);
                                } else {
                                    self.status = format!(
                                        "AutoFix: Added {} permanent override(s). Re-sorting...",
                                        added_overrides.len()
                                    );
                                    println!("[AutoFix] Added permanent overrides for: {:?}", added_overrides);
                                    
                                    // Trigger a re-sort with the new rules
                                    self.simulated_packs = None;
                                    return Task::done(Message::SmartSort);
                                }
                            } else {
                                self.status = "No packs needed override (already fixed or ignored)".to_string();
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
                        ..Default::default()
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
                log::debug!("Setting priority: {} -> {}", pack_name, score);
                Arc::make_mut(&mut self.heuristics_model.config)
                    .overrides
                    .insert(pack_name, score);
                self.heuristics_model.refresh_regex_set();
                let _ = self.heuristics_model.save();
                self.resort_scenery();
                self.sync_active_profile_scenery();
                self.editing_priority = None;
                return self.trigger_scenery_save();
            }
            Message::RemovePriority(pack_name) => {
                log::debug!("Removing priority for: {}", pack_name);
                Arc::make_mut(&mut self.heuristics_model.config)
                    .overrides
                    .remove(&pack_name);
                self.heuristics_model.refresh_regex_set();
                let _ = self.heuristics_model.save();
                self.resort_scenery();
                self.sync_active_profile_scenery();
                self.editing_priority = None;
                return self.trigger_scenery_save();
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
                                ..Default::default()
                            },
                        );

                        // Match neighbor's score exactly.
                        // Stable sort will preserve our new relative order.
                        let new_score = neighbor_score;

                        // Pin it!
                        Arc::make_mut(&mut self.heuristics_model.config)
                            .overrides
                            .insert(name.clone(), new_score);
                        self.heuristics_model.refresh_regex_set();
                        let _ = self.heuristics_model.save();

                        // Locally swap to provide instant feedback
                        Arc::make_mut(&mut self.packs).swap(idx, n_idx);
                        self.status = format!("Moved {} and pinned to score {}", name, new_score);
                        self.sync_active_profile_scenery();
                        return self.trigger_scenery_save();
                    }
                }
                Task::none()
            }
            Message::EnableAllScenery => {
                let mut new_packs = (*self.packs).clone();
                for pack in new_packs.iter_mut() {
                    if pack.status == SceneryPackType::Disabled {
                        pack.status = SceneryPackType::Active;
                    }
                }
                self.packs = Arc::new(new_packs);
                self.sync_active_profile_scenery();
                return self.trigger_scenery_save();
            }
            Message::ClearAllPins => {
                Arc::make_mut(&mut self.heuristics_model.config)
                    .overrides
                    .clear();
                self.heuristics_model.refresh_regex_set();
                let _ = self.heuristics_model.save();
                self.resort_scenery();
                self.sync_active_profile_scenery();
                self.status = "All manual reorder pins cleared".to_string();
                return self.trigger_scenery_save();
            }
            Message::WindowResized(size) => {
                self.window_size = size;
                Task::none()
            }
            Message::DragStart { index, name } => {
                self.drag_context = Some(DragContext {
                    source_index: index,
                    source_name: name,
                    hover_target_index: None,
                    cursor_position: Point::ORIGIN,
                    is_over_basket: false,
                });
                Task::none()
            }
            Message::DragBucketStart(name_opt) => {
                let name = name_opt.unwrap_or_else(|| "Selected items".to_string());
                self.drag_context = Some(DragContext {
                    source_index: usize::MAX, // Special marker for basket
                    source_name: name,
                    hover_target_index: None,
                    cursor_position: Point::ORIGIN,
                    is_over_basket: true,
                });
                Task::none()
            }
            Message::DragMove(position) => {
                if let Some(ctx) = &mut self.drag_context {
                    ctx.cursor_position = position;
                }
                Task::none()
            }
            Message::DragHover(index) => {
                if let Some(ctx) = &mut self.drag_context {
                    ctx.hover_target_index = Some(index);
                }
                Task::none()
            }
            Message::DragLeaveHover => {
                if let Some(ctx) = &mut self.drag_context {
                    ctx.hover_target_index = None;
                }
                Task::none()
            }
            Message::DragEnterBasket => {
                if let Some(ctx) = &mut self.drag_context {
                    ctx.is_over_basket = true;
                }
                Task::none()
            }
            Message::DragLeaveBasket => {
                if let Some(ctx) = &mut self.drag_context {
                    ctx.is_over_basket = false;
                }
                Task::none()
            }
            Message::DragEnd => {
                if let Some(ctx) = self.drag_context.take() {
                    if ctx.is_over_basket {
                        // Drop into basket
                        if ctx.source_index != usize::MAX {
                            // If dragging multiple items from scenery list (not currently supported by selection, 
                            // but let's assume single item for now as per ctx.source_name)
                            if !self.scenery_bucket.contains(&ctx.source_name) {
                                self.scenery_bucket.push(ctx.source_name.clone());
                                self.status = format!("Added {} to basket", ctx.source_name);
                            }
                        }
                        return Task::none();
                    }
                    if let Some(target_idx) = ctx.hover_target_index {
                        if ctx.source_index == usize::MAX {
                            return Task::done(Message::DropBucketAt(target_idx));
                        }
                        if target_idx != ctx.source_index && target_idx != ctx.source_index + 1 {
                            let packs = Arc::make_mut(&mut self.packs);
                            let name = ctx.source_name.clone();

                            // 1. Physical move
                            let item = packs.remove(ctx.source_index);
                            let actual_target = if ctx.source_index < target_idx {
                                target_idx - 1
                            } else {
                                target_idx
                            };
                            packs.insert(actual_target, item);

                            // 2. Priority pinning (Pin to neighbor score)
                            let neighbor_idx = if actual_target > 0 { actual_target - 1 } else { actual_target + 1 };
                            let mut new_score = 0;
                            if neighbor_idx < packs.len() {
                                let neighbor_name = packs[neighbor_idx].name.clone();
                                new_score = self.heuristics_model.predict(
                                    &neighbor_name,
                                    std::path::Path::new(""),
                                    &x_adox_bitnet::PredictContext {
                                        region_focus: self.region_focus.clone(),
                                        ..Default::default()
                                    },
                                );

                                Arc::make_mut(&mut self.heuristics_model.config)
                                    .overrides
                                    .insert(name.clone(), new_score);
                                self.heuristics_model.refresh_regex_set();
                                let _ = self.heuristics_model.save();
                            }

                            self.status = format!("Reordered {} (pinned to {})", name, new_score);

                            // 3. Sync to profile and save to scenery_packs.ini
                            self.sync_active_profile_scenery();
                            return self.trigger_scenery_save();
                        }
                    }
                }
                Task::none()
            }
            Message::DragCancel => {
                self.drag_context = None;
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
                let root_for_logbooks = root.clone();
                Task::batch([
                    Task::perform(async move { load_packs(root) }, Message::SceneryLoaded),
                    Task::perform(
                        async move {
                            let r = root_for_logbooks.ok_or("X-Plane root not found")?;
                            x_adox_core::logbook::LogbookParser::find_logbooks(r)
                                .map(|paths| paths.into_iter().map(LogbookPath).collect())
                                .map_err(|e| e.to_string())
                        },
                        Message::LogbooksFound,
                    ),
                ])
            }
            Message::SelectFolder => {
                self.status = "Select X-Plane folder...".to_string();
                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Select X-Plane Folder")
                            .pick_folder()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::FolderSelected,
                )
            }
            Message::FolderSelected(path_opt) => {
                if let Some(path) = path_opt {
                    // Sync current state before switching installations
                    log::debug!("Pre-syncing current state before switching installations");
                    self.sync_active_profile_scenery();
                    self.sync_active_profile_plugins();
                    self.sync_active_profile_aircraft();

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
                            
                            // Force reload of profiles for the new install location
                            if let Some(pm) = &self.profile_manager {
                                if let Ok(collection) = pm.load() {
                                    self.profiles = collection;
                                } else {
                                    self.profiles = ProfileCollection::default();
                                }
                            }

                            // Force reload of heuristics for the new install location
                            if let Some(r) = &self.xplane_root {
                                self.heuristics_model = Self::initialize_heuristics(r);
                                
                                // Apply the pins from the newly loaded profile
                                if let Some(active_name) = &self.profiles.active_profile {
                                    if let Some(profile) = self.profiles.profiles.iter().find(|p| p.name == *active_name) {
                                        let overrides = profile.scenery_overrides.iter()
                                            .map(|(k, v)| (k.clone(), *v))
                                            .collect::<std::collections::BTreeMap<_, _>>();
                                        self.heuristics_model.apply_overrides(overrides);
                                    }
                                }
                            }

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
                                Task::perform(
                                    async move {
                                        let r = root8.ok_or("X-Plane root not found")?;
                                        x_adox_core::logbook::LogbookParser::find_logbooks(r)
                                            .map(|paths| paths.into_iter().map(LogbookPath).collect::<Vec<_>>())
                                            .map_err(|e| e.to_string())
                                    },
                                    Message::LogbooksFound,
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
            Message::SelectXPlaneRoot(path) => {
                if self.xplane_root.as_ref() == Some(&path) {
                    return Task::none(); // Already selected
                }

                // Sync current state before switching installations
                log::debug!("Pre-syncing current state before switching installations");
                self.sync_active_profile_scenery();
                self.sync_active_profile_plugins();
                self.sync_active_profile_aircraft();

                self.xplane_root = Some(path.clone());
                
                self.save_app_config();
                self.load_scan_config(); // Reload exclusions/inclusions for this root
                self.profile_manager = Some(ProfileManager::new(&path));
                
                // Reload profiles for the newly selected root
                if let Some(pm) = &self.profile_manager {
                    match pm.load() {
                        Ok(collection) => {
                            self.profiles = collection;
                        }
                        Err(e) => {
                            self.status = format!("Failed to load profiles: {}", e);
                            self.profiles = ProfileCollection::default();
                        }
                    }
                }

                // Force reload of heuristics for the new install location
                if let Some(r) = &self.xplane_root {
                    self.heuristics_model = Self::initialize_heuristics(r);
                    
                    // Apply the pins from the newly loaded profile
                    if let Some(active_name) = &self.profiles.active_profile {
                        if let Some(profile) = self.profiles.profiles.iter().find(|p| p.name == *active_name) {
                            let overrides = profile.scenery_overrides.iter()
                                .map(|(k, v)| (k.clone(), *v))
                                .collect::<std::collections::BTreeMap<_, _>>();
                            log::debug!("Root switch: Applying {} overrides from profile '{}'", overrides.len(), active_name);
                            self.heuristics_model.apply_overrides(overrides);
                        }
                    }
                }

                self.status = "Loading X-Plane Profile...".to_string();

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
                    Task::perform(
                        async move {
                            let r = root8.ok_or("X-Plane root not found")?;
                            x_adox_core::logbook::LogbookParser::find_logbooks(r)
                                .map(|paths| paths.into_iter().map(LogbookPath).collect::<Vec<_>>())
                                .map_err(|e| e.to_string())
                        },
                        Message::LogbooksFound,
                    ),
                ])
            }
            Message::ToggleAircraftFolder(path) => {
                if let Some(ref mut tree) = self.aircraft_tree {
                    toggle_folder_at_path(Arc::make_mut(tree), &path);
                }
                Task::none()
            }
            Message::SelectScenery(name) => {
                let now = std::time::Instant::now();
                let is_double_click = self
                    .last_scenery_click
                    .as_ref()
                    .map(|(last_name, last_time)| {
                        last_name == &name && now.duration_since(*last_time).as_millis() < 300
                    })
                    .unwrap_or(false);

                self.last_scenery_click = Some((name.clone(), now));
                self.selected_scenery = Some(name.clone());

                // If double click, focus the map
                if is_double_click {
                    if let Some(index) = self.packs.iter().position(|p| p.name == name) {
                        if let Some(pack) = self.packs.get(index) {
                            if let Some((lat, lon)) = pack.get_centroid() {
                                // Default zoom level for focus
                                return Task::done(Message::FocusMap(lat, lon, 10.0));
                            }
                        }
                    }
                }

                Task::none()
            }
            Message::GotoScenery(name) => {
                self.active_tab = Tab::Scenery;
                self.selected_scenery = Some(name.clone());
                self.simulated_packs = None;
                self.validation_report = None;

                if let Some(index) = self.packs.iter().position(|p| p.name == name) {
                    return self.scroll_to_scenery_index(index);
                }
                Task::none()
            }
            Message::HoverScenery(name_opt) => {
                if self.hovered_scenery != name_opt {
                    self.hovered_scenery = name_opt;
                    // Reset airport hover when switching scenery packs
                    self.hovered_airport_id = None;
                }
                Task::none()
            }
            Message::HoverAirport(id_opt) => {
                if self.hovered_airport_id != id_opt {
                    self.hovered_airport_id = id_opt;
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
            Message::InstallScenery => Task::perform(pick_archive("Scenery"), |p| {
                Message::InstallPicked(Tab::Scenery, p)
            }),
            Message::InstallAircraft => Task::perform(pick_archive("Aircraft"), |p| {
                Message::InstallPicked(Tab::Aircraft, p)
            }),
            Message::InstallPlugin => Task::perform(pick_archive("Plugin"), |p| {
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
                    let model = self.heuristics_model.clone();
                    let context = x_adox_bitnet::PredictContext {
                        region_focus: self.region_focus.clone(),
                        ..Default::default()
                    };
                    self.status = format!("Installing to {:?}...", tab);
                    self.install_progress = Some(0.0);

                    return Task::run(
                        iced::stream::channel(
                            10,
                            move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                                let mut output_progress = output.clone();
                                let res = install_addon(
                                    root,
                                    zip_path,
                                    tab,
                                    None,
                                    model,
                                    context,
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
                    self.status = "Install cancelled".to_string();
                    Task::none()
                }
            }
            Message::InstallAircraftDestPicked(zip_path, dest_opt) => {
                if let Some(dest_path) = dest_opt {
                    let root = self.xplane_root.clone();
                    let model = self.heuristics_model.clone();
                    let context = x_adox_bitnet::PredictContext {
                        region_focus: self.region_focus.clone(),
                        ..Default::default()
                    };
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
                                    model,
                                    context,
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
                let (name_opt, path) = match tab {
                    Tab::Scenery => (self.selected_scenery.clone(), PathBuf::new()),
                    Tab::Aircraft => {
                        let p = self.selected_aircraft.clone();
                        let n = p.as_ref().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
                        (n, p.unwrap_or_default())
                    }
                    Tab::Plugins => {
                        let p = self.selected_plugin.clone();
                        let n = p.as_ref().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
                        (n, p.unwrap_or_default())
                    }
                    Tab::CSLs => {
                        let p = self.selected_csl.clone();
                        let n = p.as_ref().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
                        (n, p.unwrap_or_default())
                    }
                    _ => (None, PathBuf::new()),
                };

                if let Some(name) = name_opt {
                    return Task::done(Message::ShowModal(ModalState {
                        title: "Confirm Deletion".to_string(),
                        message: format!("Are you sure you want to permanently delete '{}'?", name),
                        confirm_type: ConfirmType::DeleteAddon(tab, name, path),
                        is_danger: true,
                    }));
                }
                Task::none()
            }
            Message::DeleteAddonDirect(path, tab) => {
                let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Unknown".to_string());
                Task::done(Message::ShowModal(ModalState {
                    title: "Confirm Deletion".to_string(),
                    message: format!("Are you sure you want to permanently delete '{}'?", name),
                    confirm_type: ConfirmType::DeleteAddon(tab, name, path),
                    is_danger: true,
                }))
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
            Message::ClearOverrides => {
                if let Err(e) = self.heuristics_model.clear_overrides() {
                    self.heuristics_error = Some(format!("Clear overrides failed: {}", e));
                } else {
                    let json = serde_json::to_string_pretty(self.heuristics_model.config.as_ref())
                        .unwrap_or_default();
                    self.heuristics_json = text_editor::Content::with_text(&json);
                    self.heuristics_error = None;
                    self.status = "AutoFix overrides cleared".to_string();
                }
                Task::none()
            }
            Message::ImportHeuristics => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title("Import Heuristics JSON")
                        .add_filter("JSON", &["json"])
                        .pick_file()
                        .await
                        .map(|f| f.path().to_path_buf())
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
                        rfd::AsyncFileDialog::new()
                            .set_title("Export Heuristics JSON")
                            .add_filter("JSON", &["json"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
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
                        rfd::AsyncFileDialog::new()
                            .pick_folder()
                            .await
                            .map(|f| f.path().to_path_buf())
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
                Task::perform(
                    async move {
                        log::debug!("Opening icon picker for aircraft: {:?}", path);
                        let icon_path = rfd::AsyncFileDialog::new()
                            .set_title("Select Custom Aircraft Icon")
                            .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf());
                        icon_path.map(|icon| {
                            log::info!("Selected aircraft icon: {:?}", icon);
                            (path, icon)
                        })
                    },
                    |res| {
                        if let Some((path, icon)) = res {
                            Message::IconSelected(path, icon)
                        } else {
                            log::debug!("Icon selection cancelled");
                            Message::Refresh
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
            Message::ScenerySearchChanged(query) => {
                self.scenery_search_query = query;
                if self.scenery_search_query.is_empty() {
                    self.scenery_search_matches.clear();
                    self.scenery_search_index = None;
                } else {
                    let q = self.scenery_search_query.to_lowercase();
                    self.scenery_search_matches = self
                        .packs
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| p.name.to_lowercase().contains(&q))
                        .map(|(i, _)| i)
                        .collect();

                    if self.scenery_search_matches.is_empty() {
                        self.scenery_search_index = None;
                    } else {
                        self.scenery_search_index = Some(0);
                        let target_idx = self.scenery_search_matches[0];
                        return self.scroll_to_scenery_index(target_idx);
                    }
                }
                Task::none()
            }
            Message::ScenerySearchNext => {
                if let Some(current) = self.scenery_search_index {
                    if !self.scenery_search_matches.is_empty() {
                        let next = (current + 1) % self.scenery_search_matches.len();
                        self.scenery_search_index = Some(next);
                        let target_idx = self.scenery_search_matches[next];
                        return self.scroll_to_scenery_index(target_idx);
                    }
                }
                Task::none()
            }
            Message::ScenerySearchPrev => {
                if let Some(current) = self.scenery_search_index {
                    if !self.scenery_search_matches.is_empty() {
                        let prev = if current == 0 {
                            self.scenery_search_matches.len() - 1
                        } else {
                            current - 1
                        };
                        self.scenery_search_index = Some(prev);
                        let target_idx = self.scenery_search_matches[prev];
                        return self.scroll_to_scenery_index(target_idx);
                    }
                }
                Task::none()
            }
            Message::ScenerySearchSubmit => Task::done(Message::ScenerySearchNext),
            Message::BackupUserData => {
                let xplane_root = self.xplane_root.clone();
                match xplane_root {
                    Some(root) => {
                        self.status = "Exporting configuration...".to_string();
                        Task::perform(
                            async move {
                                let handle = rfd::AsyncFileDialog::new()
                                    .set_title("Export Configuration Backup")
                                    .add_filter("X-Addon-Oxide Backup", &["xback"])
                                    .set_file_name("oxide_backup.xback")
                                    .save_file()
                                    .await;

                                if let Some(file) = handle {
                                    match x_adox_core::management::BackupManager::backup_user_data(
                                        &root,
                                        file.path(),
                                    ) {
                                        Ok(_) => Ok(file.path().to_path_buf()),
                                        Err(e) => Err(e.to_string()),
                                    }
                                } else {
                                    Err("Operation cancelled".to_string())
                                }
                            },
                            Message::BackupComplete,
                        )
                    }
                    None => Task::none(),
                }
            }
            Message::BackupComplete(result) => {
                match result {
                    Ok(path) => {
                        self.status = format!("Exported to {}", path.display());
                    }
                    Err(e) if e == "Operation cancelled" => {
                        self.status = "Export cancelled".to_string();
                    }
                    Err(e) => {
                        self.status = format!("Export failed: {}", e);
                    }
                }
                Task::none()
            }
            Message::RestoreUserData => {
                let xplane_root = self.xplane_root.clone();
                match xplane_root {
                    Some(root) => {
                        self.status = "Importing configuration...".to_string();
                        Task::perform(
                            async move {
                                let handle = rfd::AsyncFileDialog::new()
                                    .set_title("Import Configuration Backup")
                                    .add_filter("X-Addon-Oxide Backup", &["xback", "json"])
                                    .pick_file()
                                    .await;

                                if let Some(file) = handle {
                                    match x_adox_core::management::BackupManager::restore_user_data(
                                        &root,
                                        file.path(),
                                    ) {
                                        Ok(msg) => Ok(msg),
                                        Err(e) => Err(e.to_string()),
                                    }
                                } else {
                                    Err("Operation cancelled".to_string())
                                }
                            },
                            Message::RestoreComplete,
                        )
                    }
                    None => Task::none(),
                }
            }
            Message::RestoreComplete(result) => {
                match result {
                    Ok(msg) => {
                        self.status = format!("Import Success: {}", msg);
                        // Refresh app state after restore
                        Task::done(Message::Refresh)
                    }
                    Err(e) if e == "Operation cancelled" => {
                        self.status = "Import cancelled".to_string();
                        Task::none()
                    }
                    Err(e) => {
                        self.status = format!("Import failed: {}", e);
                        Task::none()
                    }
                }
            }
            Message::ModifiersChanged(modifiers) => {
                self.keyboard_modifiers = modifiers;
                Task::none()
            }
            Message::ToggleBucketItem(name) => {
                if self.keyboard_modifiers.shift() {
                    // Range selection logic
                    if let (Some(last_idx), Some(current_idx)) = (
                        self.scenery_last_bucket_index,
                        self.packs.iter().position(|p| p.name == name),
                    ) {
                        let start = last_idx.min(current_idx);
                        let end = last_idx.max(current_idx);
                        for i in start..=end {
                            let p_name = self.packs[i].name.clone();
                            if !self.scenery_bucket.contains(&p_name) {
                                self.scenery_bucket.push(p_name);
                            }
                        }
                        self.scenery_last_bucket_index = Some(current_idx);
                    } else if let Some(current_idx) = self.packs.iter().position(|p| p.name == name)
                    {
                        // Fallback to single if no last index
                        if let Some(pos) = self.scenery_bucket.iter().position(|n| n == &name) {
                            self.scenery_bucket.remove(pos);
                        } else {
                            self.scenery_bucket.push(name.clone());
                        }
                        self.scenery_last_bucket_index = Some(current_idx);
                    }
                } else {
                    // Standard toggle
                    if let Some(pos) = self.scenery_bucket.iter().position(|n| n == &name) {
                        self.scenery_bucket.remove(pos);
                    } else {
                        self.scenery_bucket.push(name.clone());
                    }
                    if let Some(idx) = self.packs.iter().position(|p| p.name == name) {
                        self.scenery_last_bucket_index = Some(idx);
                    }
                }
                Task::none()
            }
            Message::ClearBucket => {
                self.scenery_bucket.clear();
                self.scenery_last_bucket_index = None;
                self.selected_basket_items.clear();
                Task::none()
            }
            Message::ToggleBucket => {
                self.show_scenery_basket = !self.show_scenery_basket;
                Task::none()
            }
            Message::ToggleBasketSelection(name) => {
                if self.selected_basket_items.contains(&name) {
                    self.selected_basket_items.remove(&name);
                } else {
                    self.selected_basket_items.insert(name.clone());
                }
                Task::none()
            }
            Message::ToggleAutopin(enabled) => {
                self.autopin_enabled = enabled;
                Task::none()
            }
            Message::BasketDragStart => {
                self.is_basket_dragging = true;
                self.basket_drag_origin = None;
                Task::none()
            }
            Message::BasketDragged(pos) => {
                if let Some(origin) = self.basket_drag_origin {
                    let delta = pos - origin;
                    // For Right margin: moving mouse RIGHT (delta.x > 0) decreases margin
                    self.basket_offset.x = (self.basket_offset.x - delta.x).max(0.0);
                    // For Top margin: moving mouse DOWN (delta.y > 0) increases margin
                    self.basket_offset.y = (self.basket_offset.y + delta.y).max(0.0);
                    
                    // Clamp to screen
                    self.basket_offset.x = self.basket_offset.x.min(self.window_size.width - self.basket_size.x);
                    self.basket_offset.y = self.basket_offset.y.min(self.window_size.height - self.basket_size.y);
                    
                    self.basket_drag_origin = Some(pos);
                } else {
                    self.basket_drag_origin = Some(pos);
                }
                Task::none()
            }
            Message::BasketDragEnd => {
                self.is_basket_dragging = false;
                self.basket_drag_origin = None;
                Task::none()
            }
            Message::BasketResizeStart(edge) => {
                self.active_resize_edge = Some(edge);
                self.basket_drag_origin = None;
                Task::none()
            }
            Message::BasketResized(pos) => {
                if let (Some(origin), Some(edge)) = (self.basket_drag_origin, self.active_resize_edge) {
                    let delta = pos - origin;

                    match edge {
                        ResizeEdge::Top => {
                            let new_height = (self.basket_size.y - delta.y).max(150.0);
                            let actual_dy = self.basket_size.y - new_height;
                            self.basket_size.y = new_height;
                            self.basket_offset.y += actual_dy;
                        }
                        ResizeEdge::Bottom => {
                            self.basket_size.y = (self.basket_size.y + delta.y).max(150.0);
                        }
                        ResizeEdge::Left => {
                            self.basket_size.x = (self.basket_size.x - delta.x).max(200.0);
                        }
                        ResizeEdge::Right => {
                            let new_width = (self.basket_size.x + delta.x).max(200.0);
                            let actual_dx = new_width - self.basket_size.x;
                            self.basket_size.x = new_width;
                            self.basket_offset.x -= actual_dx;
                        }
                        ResizeEdge::TopLeft => {
                            // Top
                            let new_height = (self.basket_size.y - delta.y).max(150.0);
                            let actual_dy = self.basket_size.y - new_height;
                            self.basket_size.y = new_height;
                            self.basket_offset.y += actual_dy;
                            // Left
                            self.basket_size.x = (self.basket_size.x - delta.x).max(200.0);
                        }
                        ResizeEdge::TopRight => {
                            // Top
                            let new_height = (self.basket_size.y - delta.y).max(150.0);
                            let actual_dy = self.basket_size.y - new_height;
                            self.basket_size.y = new_height;
                            self.basket_offset.y += actual_dy;
                            // Right
                            let new_width = (self.basket_size.x + delta.x).max(200.0);
                            let actual_dx = new_width - self.basket_size.x;
                            self.basket_size.x = new_width;
                            self.basket_offset.x -= actual_dx;
                        }
                        ResizeEdge::BottomLeft => {
                            // Bottom
                            self.basket_size.y = (self.basket_size.y + delta.y).max(150.0);
                            // Left
                            self.basket_size.x = (self.basket_size.x - delta.x).max(200.0);
                        }
                        ResizeEdge::BottomRight => {
                            // Bottom
                            self.basket_size.y = (self.basket_size.y + delta.y).max(150.0);
                            // Right
                            let new_width = (self.basket_size.x + delta.x).max(200.0);
                            let actual_dx = new_width - self.basket_size.x;
                            self.basket_size.x = new_width;
                            self.basket_offset.x -= actual_dx;
                        }
                    }
                    
                    // Clamping
                    self.basket_offset.x = self.basket_offset.x.max(0.0).min(self.window_size.width - self.basket_size.x);
                    self.basket_offset.y = self.basket_offset.y.max(0.0).min(self.window_size.height - self.basket_size.y);

                    self.basket_drag_origin = Some(pos);
                } else {
                    self.basket_drag_origin = Some(pos);
                }
                Task::none()
            }
            Message::BasketResizeEnd => {
                self.active_resize_edge = None;
                self.basket_drag_origin = None;
                Task::none()
            }
            Message::DropBucketAt(target_idx) => {
                let items_to_move = if self.selected_basket_items.is_empty() {
                    self.scenery_bucket.clone()
                } else {
                    self.scenery_bucket
                        .iter()
                        .filter(|name| self.selected_basket_items.contains(*name))
                        .cloned()
                        .collect()
                };

                if items_to_move.is_empty() {
                    return Task::none();
                }

                let mut sm = x_adox_core::scenery::SceneryManager::new(std::path::PathBuf::new());
                sm.packs = (*self.packs).clone();
                
                sm.drop_basket_at(
                    &items_to_move,
                    target_idx,
                    &mut self.heuristics_model,
                    &x_adox_bitnet::PredictContext {
                        region_focus: self.region_focus.clone(),
                        ..Default::default()
                    },
                    self.autopin_enabled,
                );

                self.packs = Arc::new(sm.packs);
                self.scenery_bucket.retain(|name| !items_to_move.contains(name));
                self.selected_basket_items.clear();
                self.scenery_last_bucket_index = None;
                
                self.status = "Applied!".to_string();
                self.sync_active_profile_scenery();
                self.trigger_scenery_save()
            }

            Message::Tick(_now) => {
                let mut tasks = Vec::new();

                if self.loading_state.is_loading {
                    self.animation_time += 0.05;
                    if self.animation_time > 1000.0 {
                        self.animation_time = 0.0;
                    }
                }

                if let Some(ctx) = &self.drag_context {
                    if !ctx.is_over_basket {
                        let top_threshold = 50.0;
                        let bottom_threshold = self.window_size.height - 50.0;
                        let scroll_speed = 15.0;

                        if ctx.cursor_position.y < top_threshold {
                            tasks.push(scrollable::scroll_by(
                                self.scenery_scroll_id.clone(),
                                scrollable::AbsoluteOffset {
                                    x: 0.0,
                                    y: -scroll_speed,
                                },
                            ));
                        } else if ctx.cursor_position.y > bottom_threshold {
                            tasks.push(scrollable::scroll_by(
                                self.scenery_scroll_id.clone(),
                                scrollable::AbsoluteOffset {
                                    x: 0.0,
                                    y: scroll_speed,
                                },
                            ));
                        }
                    }
                }

                if tasks.is_empty() {
                    Task::none()
                } else {
                    Task::batch(tasks)
                }
            }

            Message::ShowModal(state) => {
                self.active_modal = Some(state);
                Task::none()
            }
            Message::CloseModal => {
                self.active_modal = None;
                Task::none()
            }
            Message::ConfirmModal(confirm_type) => {
                self.active_modal = None;
                match confirm_type {
                    ConfirmType::DeleteAddon(tab, _name, path) => {
                        // For direct deletion, we ensure selection is set so ConfirmDelete works
                        match tab {
                            Tab::Scenery => {
                                // Scenery deletion uses selected_scenery string
                                self.selected_scenery = Some(_name);
                            }
                            Tab::Aircraft => self.selected_aircraft = Some(path),
                            Tab::Plugins => self.selected_plugin = Some(path),
                            Tab::CSLs => self.selected_csl = Some(path),
                            _ => {}
                        }
                        Task::done(Message::ConfirmDelete(tab, true))
                    }
                    ConfirmType::BulkDeleteLogbook => Task::done(Message::ConfirmLogbookBulkDelete),
                    ConfirmType::BulkDeleteLogIssues => {
                        // Implement bulk delete for log issues if needed
                        Task::none()
                    }
                }
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        use iced::event::{self, Event};
        use iced::keyboard;
        use iced::mouse;
        use iced::time;

        let dragging = if self.drag_context.is_some() {
            event::listen_with(|event, _status, _window| match event {
                Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    Some(Message::DragMove(position))
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    Some(Message::DragEnd)
                }
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(keyboard::key::Named::Escape),
                    ..
                }) => Some(Message::DragCancel),
                _ => None,
            })
        } else {
            Subscription::none()
        };

        let window_sub = event::listen_with(|event, _status, _window| match event {
            Event::Window(iced::window::Event::Resized(size)) => Some(Message::WindowResized(size)),
            _ => None,
        });

        let basket_dragging = if self.is_basket_dragging {
            event::listen_with(|event, _status, _window| match event {
                Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    Some(Message::BasketDragged(position))
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    Some(Message::BasketDragEnd)
                }
                _ => None,
            })
        } else {
            Subscription::none()
        };

        let basket_resizing = if self.active_resize_edge.is_some() {
            event::listen_with(|event, _status, _window| match event {
                Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    Some(Message::BasketResized(position))
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    Some(Message::BasketResizeEnd)
                }
                _ => None,
            })
        } else {
            Subscription::none()
        };

        let tick = if self.loading_state.is_loading
            || self.drag_context.is_some()
            || self.is_basket_dragging
            || self.active_resize_edge.is_some()
        {
            time::every(std::time::Duration::from_millis(16)).map(Message::Tick)
        } else {
            Subscription::none()
        };

        let kb_sub = event::listen_with(|event, _status, _window| match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                Some(Message::ModifiersChanged(modifiers))
            }
            _ => None,
        });

        Subscription::batch(vec![
            dragging,
            basket_dragging,
            basket_resizing,
            tick,
            kb_sub,
            window_sub,
        ])
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
            let mut final_view = stack![main_view];

            if self.show_scenery_basket {
                let basket = self.view_scenery_basket();

                final_view = final_view.push(
                    container(basket)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Right)
                        .align_y(iced::alignment::Vertical::Top)
                        .padding(Padding {
                            top: self.basket_offset.y,
                            right: self.basket_offset.x,
                            bottom: 0.0,
                            left: 0.0,
                        })
                );
            }

            if let Some(ctx) = &self.drag_context {
                let ghost = container(self.view_ghost(ctx))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(Padding {
                        top: ctx.cursor_position.y - 10.0,
                        right: 0.0,
                        bottom: 0.0,
                        left: ctx.cursor_position.x - 10.0,
                    });

                final_view = final_view.push(ghost);
            }

            if let Some(modal) = &self.active_modal {
                let modal_content = container(self.view_modal(modal))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .style(|_| container::Style {
                        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.85))),
                        ..Default::default()
                    });

                final_view = final_view.push(modal_content);
            }

            final_view.into()
        }
    }

    fn view_modal<'a>(&self, modal: &'a ModalState) -> Element<'a, Message> {
        let title = text(&modal.title).size(20).color(Color::WHITE);
        let message = text(&modal.message).size(16).color(style::palette::TEXT_SECONDARY);

        let confirm_btn = button(text("Confirm").size(14))
            .on_press(Message::ConfirmModal(modal.confirm_type.clone()))
            .style(if modal.is_danger {
                style::button_danger
            } else {
                style::button_primary
            })
            .padding([10, 25]);

        let cancel_btn = button(text("Cancel").size(14))
            .on_press(Message::CloseModal)
            .style(style::button_secondary)
            .padding([10, 20]);

        container(
            column![
                title,
                message,
                row![cancel_btn, confirm_btn].spacing(15).align_y(iced::Alignment::Center)
            ]
            .spacing(20)
            .padding(30)
            .width(Length::Fixed(450.0))
        )
        .style(style::container_card)
        .into()
    }

    fn view_ghost(&self, ctx: &DragContext) -> Element<'static, Message> {
        container(
            row![
                svg(self.icon_grip.clone())
                    .width(Length::Fixed(16.0))
                    .height(Length::Fixed(16.0))
                    .style(|_, _| svg::Style {
                        color: Some(style::palette::TEXT_PRIMARY),
                    }),
                text(ctx.source_name.clone()).size(14)
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        )
        .padding(15)
        .style(style::container_ghost)
        .width(Length::Fixed(300.0))
        .into()
    }

    fn view_loading_overlay(&self) -> Element<'_, Message> {
        let p = self.loading_state.progress();
        let percentage = (p * 100.0) as u32;

        let title = text("X-Addon-Oxide")
            .size(32)
            .color(style::palette::TEXT_PRIMARY);

        // Pulsing glow for the title
        let pulse = (self.animation_time.sin() * 0.5 + 0.5) * 0.5 + 0.5;
        let title_color = Color {
            a: pulse,
            ..style::palette::TEXT_PRIMARY
        };
        // Floating animation for the title
        let float_offset = (self.animation_time * 0.5).sin() * 5.0;
        let title = title.color(title_color);

        let subtitle = text("Synchronizing Simulation Environment...")
            .size(14)
            .color(style::palette::TEXT_SECONDARY);

        let items = [
            ("Scenery Library", self.loading_state.scenery),
            ("Aircraft Addons", self.loading_state.aircraft),
            (
                "Plugins & CSLs",
                self.loading_state.plugins && self.loading_state.csls,
            ),
            ("Airport Database", self.loading_state.airports),
            ("Pilot Logbook", self.loading_state.logbook),
        ];

        let mut status_grid = Column::new().spacing(10).width(Length::Fixed(300.0));
        for (label, done) in items {
            status_grid = status_grid.push(
                row![
                    text(label).size(12).color(if done {
                        style::palette::TEXT_PRIMARY
                    } else {
                        style::palette::TEXT_SECONDARY
                    }),
                    iced::widget::horizontal_space(),
                    if done {
                        text("COMPLETE")
                            .size(10)
                            .color(style::palette::ACCENT_GREEN)
                    } else {
                        // Pulse the "LOADING..." text
                        let alpha = (self.animation_time * 3.0).sin() * 0.3 + 0.7;
                        text("LOADING...")
                            .size(10)
                            .color(Color { a: alpha, ..style::palette::ACCENT_BLUE })
                    }
                ]
                .align_y(iced::Alignment::Center),
            );
        }

        container(
            column![
                // Large pulsing logo with glow
                container(
                    svg(self.icon_aircraft.clone())
                        .width(80)
                        .height(80)
                        .style(move |_, _| svg::Style {
                            color: Some(Color {
                                a: (self.animation_time * 2.0).sin() * 0.2 + 0.6,
                                ..style::palette::ACCENT_BLUE
                            }),
                        })
                )
                .padding(30),
                column![
                    container(title).padding(Padding { top: float_offset, bottom: -float_offset, ..Default::default() }),
                    subtitle
                ]
                .align_x(iced::Alignment::Center)
                .spacing(5),
                iced::widget::vertical_space().height(40),
                container(
                    column![
                        row![
                            text("Overall Progress")
                                .size(12)
                                .color(style::palette::TEXT_PRIMARY),
                            iced::widget::horizontal_space(),
                            text(format!("{}%", percentage))
                                .size(12)
                                .color(style::palette::TEXT_PRIMARY),
                        ],
                        progress_bar(0.0..=1.0, p)
                            .height(10)
                            .style(move |_theme: &Theme| {
                                // Moving shimmer effect
                                let shimmer = ((self.animation_time * 4.0).sin() * 0.5 + 0.5) * 0.3;
                                let bar_color = if p < 0.01 {
                                    // Pulse the empty bar to show it's "alive"
                                    Color {
                                        r: style::palette::ACCENT_BLUE.r + shimmer,
                                        g: style::palette::ACCENT_BLUE.g + shimmer,
                                        b: style::palette::ACCENT_BLUE.b + shimmer,
                                        a: 0.2 + shimmer,
                                    }
                                } else {
                                    style::palette::ACCENT_BLUE
                                };

                                progress_bar::Style {
                                    background: Background::Color(style::palette::SURFACE_VARIANT),
                                    bar: Background::Color(bar_color),
                                    border: Border {
                                        radius: 5.0.into(),
                                        ..Default::default()
                                    },
                                }
                            }),
                    ]
                    .spacing(12)
                )
                .width(Length::Fixed(400.0)),
                iced::widget::vertical_space().height(40),
                status_grid,
            ]
            .align_x(iced::Alignment::Center)
            .max_width(600),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(move |_| {
            // Subtle breathing background with slightly deeper tones
            let depth = (self.animation_time * 0.15).sin() * 0.005 + 0.1;
            container::Style {
                background: Some(Background::Color(Color::from_rgb(depth, depth, depth))),
                ..Default::default()
            }
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
                                        button(text("Go to").size(11))
                                            .on_press(Message::GotoScenery(issue.pack_name.clone()))
                                            .style(style::button_secondary)
                                            .padding([4, 8]),
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
                                        .on_press(Message::GotoScenery(first.pack_name.clone()))
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

    fn view_companion_apps(&self) -> Element<'_, Message> {
        let title = text("Companion Apps").size(18);

        let app_selector = if self.companion_apps.is_empty() {
            container(
                text("No apps added")
                    .size(14)
                    .color(style::palette::TEXT_SECONDARY),
            )
            .padding(10)
        } else {
            let selected = self
                .selected_companion_app
                .and_then(|idx| self.companion_apps.get(idx).map(|a| a.name.clone()));

            container(
                row![
                    pick_list(
                        self.companion_apps
                            .iter()
                            .map(|a| a.name.clone())
                            .collect::<Vec<_>>(),
                        selected,
                        |name| {
                            let idx = self
                                .companion_apps
                                .iter()
                                .position(|a| a.name == name)
                                .unwrap_or(0);
                            Message::SelectCompanionApp(idx)
                        }
                    )
                    .width(Length::Fill)
                    .placeholder("Select App..."),
                    button(text("Launch").size(14))
                        .on_press(Message::LaunchCompanionApp)
                        .style(style::button_primary)
                        .padding([8, 20]),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center),
            )
        };

        let manage_button = button(
            text(if self.show_companion_manage {
                "Hide Manager"
            } else {
                "Manage Apps..."
            })
            .size(12),
        )
        .on_press(Message::ToggleCompanionManager)
        .style(style::button_secondary)
        .padding([5, 10]);

        let mut content = Column::new()
            .spacing(15)
            .push(
                row![title, iced::widget::horizontal_space(), manage_button]
                    .align_y(iced::Alignment::Center),
            )
            .push(app_selector);

        if self.show_companion_manage {
            let mut apps_list = Column::new().spacing(10);

            if !self.companion_apps.is_empty() {
                apps_list = apps_list.push(
                    row![
                        text("Launch with X-Plane")
                            .size(10)
                            .color(style::palette::TEXT_SECONDARY)
                            .width(Length::Fixed(120.0)),
                        text("Application")
                            .size(10)
                            .color(style::palette::TEXT_SECONDARY),
                    ]
                    .spacing(15)
                    .padding(Padding {
                        top: 0.0,
                        right: 0.0,
                        bottom: 5.0,
                        left: 0.0,
                    }),
                );
            }

            for (idx, app) in self.companion_apps.iter().enumerate() {
                apps_list = apps_list.push(
                    row![
                        container(
                            tooltip(
                                checkbox("", app.auto_launch)
                                    .on_toggle(move |_| Message::ToggleCompanionAutoLaunch(idx))
                                    .size(16),
                                "Automatically launch this application when X-Plane starts",
                                tooltip::Position::Top,
                            )
                            .style(style::container_tooltip)
                        )
                        .width(Length::Fixed(120.0))
                        .center_x(Length::Fixed(60.0)),
                        column![
                            text(&app.name).size(14),
                            text(app.path.to_string_lossy())
                                .size(10)
                                .color(style::palette::TEXT_SECONDARY),
                        ]
                        .width(Length::Fill),
                        button(svg(self.icon_trash.clone()).width(14).height(14))
                            .on_press(Message::RemoveCompanionApp(idx))
                            .style(style::button_secondary)
                            .padding(8),
                    ]
                    .spacing(15)
                    .align_y(iced::Alignment::Center),
                );
            }

            let add_form = container(
                column![
                    text("Add New Companion App").size(14),
                    row![
                        text_input("Application Name", &self.new_companion_name)
                            .on_input(Message::UpdateCompanionNameInput)
                            .padding(8),
                        button(text("Browse...").size(12))
                            .on_press(Message::BrowseForCompanionPath)
                            .style(style::button_secondary)
                            .padding([8, 15]),
                    ]
                    .spacing(10),
                    if let Some(path) = &self.new_companion_path {
                        text(format!("Selected: {}", path.display()))
                            .size(10)
                            .color(style::palette::ACCENT_GREEN)
                    } else {
                        text("No executable selected")
                            .size(10)
                            .color(style::palette::TEXT_SECONDARY)
                    },
                    container(
                        button(
                            text("Add Application")
                                .size(14)
                                .width(Length::Fill)
                                .align_x(iced::alignment::Horizontal::Center)
                                .color(Color::WHITE),
                        )
                        .on_press(Message::AddCompanionApp)
                        .style(style::button_primary)
                        .padding([10, 20])
                        .width(Length::Fill),
                    )
                    .width(Length::Fill)
                    .padding([10, 0]),
                ]
                .spacing(10),
            )
            .padding(20)
            .style(style::container_sidebar);

            content = content.push(
                container(
                    column![
                        if !self.companion_apps.is_empty() {
                            Element::from(container(apps_list).padding(10))
                        } else {
                            Element::from(iced::widget::Space::with_height(0.0))
                        },
                        add_form
                    ]
                    .spacing(15),
                )
                .padding(10)
                .style(style::container_card),
            );
        }

        container(content)
            .padding(20)
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

        let filter_bar = if self.logbook_expanded {
            let logbook_selector = if !self.available_logbooks.is_empty() {
                container(
                    row![
                        text("Select Logbook:").size(14).color(style::palette::TEXT_SECONDARY),
                        pick_list(
                            self.available_logbooks.as_slice(),
                            self.selected_logbook_path.clone(),
                            Message::SelectLogbook,
                        )
                        .placeholder("Select logbook...")
                        .text_size(14)
                        .padding(5)
                        .style(style::pick_list_primary),
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center)
                )
            } else {
                container(text("No logbooks found in Output/logbooks").size(14).color(style::palette::ACCENT_RED))
            };

            container(
                column![
                    logbook_selector,
                    row![
                        text_input(
                            "Filter Aircraft (Tail/Type)...",
                            &self.logbook_filter_aircraft
                        )
                        .on_input(Message::LogbookFilterAircraftChanged)
                        .padding(8)
                        .size(14)
                        .style(style::text_input_primary),
                        checkbox("Circular Only", self.logbook_filter_circular)
                            .on_toggle(Message::LogbookFilterCircularToggled)
                            .size(16),
                    ]
                    .spacing(20)
                    .align_y(iced::Alignment::Center),
                    row![
                        text("Duration (h):")
                            .size(14)
                            .color(style::palette::TEXT_SECONDARY),
                        text_input("Min", &self.logbook_filter_duration_min)
                            .on_input(Message::LogbookFilterDurationMinChanged)
                            .padding(6)
                            .width(Length::Fixed(60.0))
                            .size(12)
                            .style(style::text_input_primary),
                        text("to").size(12).color(style::palette::TEXT_SECONDARY),
                        text_input("Max", &self.logbook_filter_duration_max)
                            .on_input(Message::LogbookFilterDurationMaxChanged)
                            .padding(6)
                            .width(Length::Fixed(60.0))
                            .size(12)
                            .style(style::text_input_primary),
                        iced::widget::horizontal_space(),
                        button(text("Clear Filters").size(12))
                            .on_press(Message::LogbookFilterAircraftChanged(String::new())) // Hacky clear
                            .style(style::button_secondary)
                            .padding([5, 10]),
                    ]
                    .spacing(15)
                    .align_y(iced::Alignment::Center),
                    row![
                        text("Bulk Actions:")
                            .size(14)
                            .color(style::palette::TEXT_SECONDARY),
                        checkbox("Select All (filtered)", {
                            let filtered = self.filtered_logbook_indices();
                            !filtered.is_empty() && filtered.iter().all(|idx| self.logbook_selection.contains(idx))
                        })
                        .on_toggle(Message::ToggleAllLogbookSelection)
                        .size(16),
                        iced::widget::horizontal_space(),
                        if !self.logbook_selection.is_empty() {
                            Element::from(
                                button(
                                    text(format!(
                                        "Delete Selected ({})",
                                        self.logbook_selection.len()
                                    ))
                                    .size(12),
                                )
                                .on_press(Message::RequestLogbookBulkDelete)
                                .style(style::button_danger)
                                .padding([5, 15]),
                            )
                        } else {
                            Element::from(iced::widget::Space::with_height(0.0))
                        },
                    ]
                    .spacing(20)
                    .align_y(iced::Alignment::Center),
                ]
                .spacing(10),
            )
            .padding(10)
            .style(style::container_sidebar)
        } else {
            container(column![])
        };

        let log_list_content: Element<'_, Message> = if !self.logbook_expanded {
            container(column![]).into()
        } else if self.logbook.is_empty() {
            container(text("No logbook entries found or logbook not loaded.").size(14))
                .center_x(Length::Fill)
                .padding(20)
                .into()
        } else {
            let filtered_indices = self.filtered_logbook_indices();

            if filtered_indices.is_empty() {
                container(text("No entries match filters.").size(14))
                    .center_x(Length::Fill)
                    .padding(20)
                    .into()
            } else {
                let mut col = Column::new().spacing(5);
                for idx in filtered_indices {
                    let entry = &self.logbook[idx];
                    let is_selected = self.selected_flight == Some(idx);

                    let date_str = entry
                        .date
                        .map(|d: chrono::NaiveDate| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_else(|| "Unknown Date".to_string());

                    let is_checked = self.logbook_selection.contains(&idx);
                    let row_content = row![
                        checkbox("", is_checked)
                            .on_toggle(move |_| Message::ToggleLogbookEntrySelection(idx))
                            .size(16),
                        text(date_str).width(Length::Fixed(90.0)).size(12),
                        text(&entry.dep_airport).width(Length::Fixed(50.0)).size(12),
                        text("->").width(Length::Fixed(20.0)).size(12),
                        text(&entry.arr_airport).width(Length::Fixed(50.0)).size(12),
                        text(&entry.aircraft_type)
                            .width(Length::Fixed(70.0))
                            .size(12),
                        text(format!("{:.1}h", entry.total_duration))
                            .width(Length::Fixed(40.0))
                            .size(12),
                        iced::widget::horizontal_space(),
                        button(svg(self.icon_trash.clone()).width(12).height(12))
                            .on_press(Message::DeleteLogbookEntry(idx))
                            .style(style::button_danger)
                            .padding(6),
                    ]
                    .spacing(8)
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
                container(col).into()
            }
        };

        scrollable(
            column![
                self.view_companion_apps(),
                logbook_header,
                filter_bar,
                log_list_content,
            ]
            .spacing(15),
        )
        .width(Length::Fill)
        .height(Length::Fill)
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
                .push(self.sidebar_button("Settings", Tab::Settings))
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

        let delete_btn = button(
            text("Delete...")
                .size(12)
                .align_x(iced::alignment::Horizontal::Center),
        )
        .padding([6, 12]);

        let delete_btn = if has_selection {
            delete_btn
                .on_press(delete_msg)
                .style(style::button_danger)
        } else {
            delete_btn.style(style::button_secondary)
        };

        let refresh_btn = button(
            row![
                svg(self.refresh_icon.clone())
                    .width(14)
                    .height(14)
                    .style(|_, _| svg::Style {
                        color: Some(Color::WHITE),
                    }),
                text("Refresh").size(12),
                horizontal_space().width(22)
            ]
            .spacing(8)
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
                    text("Settings").size(12),
                    horizontal_space().width(22)
                ]
                .spacing(8)
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
                    text(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .size(12)
                        .color(style::palette::TEXT_SECONDARY),
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
                        text_input("Launch args (e.g. --safe_mode=PLG)", &self.launch_args)
                            .on_input(Message::LaunchArgsChanged)
                            .size(12)
                            .width(Length::Fixed(200.0))
                            .style(style::text_input_primary),
                        button(text("?").size(12).color(Color::WHITE))
                            .on_press(Message::OpenLaunchHelp)
                            .style(style::button_secondary)
                            .padding([4, 8]),
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
        } else if self.show_launch_help {
            stack![
                main_content,
                container(self.view_launch_help())
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
        } else if self.show_logbook_bulk_delete_confirm {
            stack![
                main_content,
                container(self.view_logbook_bulk_delete_dialog())
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

    fn view_launch_help(&self) -> Element<'_, Message> {
        let help_text = vec![
            (
                "General Options",
                vec![
                    ("--help, -h", "Prints listing of all command-line options, then quits."),
                    ("--no_sound", "Runs without initializing sound - can help identify a problem with sound hardware."),
                    ("--no_joysticks", "Runs without initializing joysticks, yokes, pedals, or other USB hardware."),
                    ("--missing_strings", "Output missing localizations to the log file (English native)."),
                    ("--lang=<lang code>", "Runs the sim in a specific language (bypasses system detection)."),
                    ("--disable_networking", "Prevents X-Plane from sending or receiving data over the network."),
                    ("--allow_rosetta", "Allow PPC emulation on x86 machines (Mac, discouraged)."),
                ],
            ),
            (
                "Auto-Configure X-Plane",
                vec![
                    ("--pref:<key>=<value>", "Sets an individual preference to an overloaded value."),
                    ("--dref:<ref>=<value>", "Sets a dataref to a value at startup (sim-created only)."),
                ],
            ),
            (
                "Disable Hardware Acceleration",
                vec![
                    ("--no_vbos", "Disables the use of vertex buffer objects."),
                    ("--no_fbos", "Disable the use of framebuffer objects."),
                    ("--no_pbos", "Disable the use of pixelbuffer objects."),
                    ("--no_sprites", "Disables the use of point sprites (accelerates runway lights)."),
                    ("--no_pixel_counters", "Disables pixel counters (used for sun glare)."),
                    ("--no_aniso_filtering", "Disables anisotropic filtering of textures."),
                    ("--no_hw_mipmap", "Disables hardware accelerated mipmap-creation (CPU instead)."),
                    ("--no_fshaders", "Disable the use of fragment shaders."),
                    ("--no_vshaders", "Disable the use of vertex shaders."),
                    ("--no_glsl", "Disable the use of GLSL shaders."),
                    ("--limited_glsl", "Force shaders to act as if graphics hardware is limited."),
                    ("--unlimited_glsl", "Override detection to run advanced shaders on old machines (can crash)."),
                    ("--no_threaded_ogl", "Disable the use of OpenGL via multiple threads."),
                ],
            ),
            (
                "Enable Incompatible Hardware Acceleration",
                vec![
                    ("--use_vbos", "Forces the use of VBOs."),
                    ("--use_sprites", "Forces the use of point sprites."),
                    ("--use_fshaders", "Force the use of fragment shaders."),
                    ("--use_vshaders", "Force the use of vertex shaders."),
                    ("--use_glsl", "Force the use of GLSL."),
                    ("--use_fbos", "Force the use of FBOs."),
                    ("--force_run", "Allow X-Plane to run even if minimum requirements aren't met."),
                    ("--fake_vr", "Enable VR from settings and see 2D representation on monitor."),
                ],
            ),
            (
                "Windowing Options",
                vec![
                    ("--full=<width>x<height>", "Launches in full-screen mode at specific resolution."),
                    ("--window=<width>x<height>", "Launches in a window with specified width & height."),
                ],
            ),
            (
                "Improving Reproducibility",
                vec![
                    ("--weather_seed=<number>", "Seeds the weather system random number generator."),
                    ("--time_seed=<number>", "Seeds the non-weather systems random number generator."),
                    ("--safe_mode=[...]", "Runs in safe mode: GFX, PLG, SCN, ART, UI (comma separated)."),
                ],
            ),
            (
                "Framerate Test",
                vec![
                    ("--fps_test=n", "Runs a frame-rate test (3 digits: angle, complexity, weather)."),
                    ("--verbose", "Include detailed timing information for every frame in FPS test."),
                    ("--require_fps=n", "Fail/pass mode: sim exits with 0 if FPS > N, else 1."),
                    ("--qa_script=<file>", "Use text file to run time-based performance monitoring."),
                ],
            ),
        ];

        let mut content_col = column![row![
            text("Launch Arguments Reference").size(24),
            iced::widget::horizontal_space(),
            button(text("X").size(18).color(Color::WHITE))
                .on_press(Message::CloseLaunchHelp)
                .style(style::button_danger)
                .padding([5, 10]),
        ]
        .align_y(iced::Alignment::Center),]
        .spacing(20);

        for (section_title, options) in help_text {
            content_col = content_col.push(
                text(section_title)
                    .size(18)
                    .color(style::palette::ACCENT_BLUE),
            );

            let mut options_col = column![].spacing(8).padding(Padding {
                left: 10.0,
                ..Padding::ZERO
            });
            for (flag, desc) in options {
                options_col = options_col.push(
                    row![
                        text(flag)
                            .width(Length::Fixed(180.0))
                            .color(style::palette::ACCENT_PURPLE)
                            .size(12),
                        text(desc).width(Length::Fill).size(12),
                    ]
                    .spacing(10),
                );
            }
            content_col = content_col.push(options_col);
        }

        let scrollable_content = scrollable(content_col).height(Length::Fixed(500.0));

        container(
            column![
                scrollable_content,
                button(text("Close").size(14))
                    .on_press(Message::CloseLaunchHelp)
                    .style(style::button_premium_glow)
                    .padding([10, 20]),
            ]
            .spacing(20)
            .align_x(iced::Alignment::Center),
        )
        .style(style::container_modal)
        .padding(30)
        .width(Length::Fixed(720.0))
        .into()
    }

    fn view_logbook_bulk_delete_dialog(&self) -> Element<'_, Message> {
        let count = self.logbook_selection.len();
        let content = column![
            text("Confirm Bulk Deletion").size(24).color(Color::WHITE),
            text(format!(
                "Are you sure you want to delete {} selected logbook entries?",
                count
            ))
            .size(16)
            .color(style::palette::TEXT_PRIMARY),
            text("This action cannot be undone. A backup of your logbook will be created.")
                .size(14)
                .color(style::palette::TEXT_SECONDARY),
            row![
                button(text("Cancel").size(14))
                    .on_press(Message::CancelLogbookBulkDelete)
                    .style(style::button_secondary)
                    .padding([10, 20]),
                button(text(format!("Delete {} Entries", count)).size(14))
                    .on_press(Message::ConfirmLogbookBulkDelete)
                    .style(style::button_danger)
                    .padding([10, 30]),
            ]
            .spacing(20)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(20)
        .padding(30)
        .width(Length::Fixed(500.0));

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
                hovered_airport_id: self.hovered_airport_id.as_ref(),
                tile_manager: &self.tile_manager,
                zoom,
                center: self.map_center,
                airports: &self.airports,
                selected_flight: self.selected_flight.and_then(|idx| self.logbook.get(idx)),
                filters: &self.map_filters,
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
                    text("12 - Orbx A Custom").size(12),
                    text("20 - Global Airports").size(12),
                    text("25 - Landmarks").size(12),
                    text("28 - Orbx B / TrueEarth").size(12),
                    text("30 - SimHeaven / X-World").size(12),
                    text("32 - Global Forests").size(12),
                    text("40 - Default (fallback)").size(12),
                    text("44 - Birds").size(12),
                    text("45 - Libraries").size(12),
                    text("48 - AutoOrtho Overlays").size(12),
                    text("58 - Ortho / Photo").size(12),
                    text("60 - Mesh / Terrain").size(12),
                    text("95 - AutoOrtho Base").size(12),
                ]
                .spacing(6)
            } else {
                column![
                    text("Inspector Panel").size(18),
                    container(if let Some(airport_id) = self.hovered_airport_id.as_ref() {
                        if let Some(airport) = self.airports.get(airport_id) {
                            column![
                                text(format!("Airport: {}", airport.id)).size(20).color(style::palette::ACCENT_BLUE),
                                text(&airport.name).size(14).color(style::palette::TEXT_PRIMARY),
                                text(format!("Type: {:?}", airport.airport_type)).size(12).color(style::palette::TEXT_SECONDARY),
                                if let (Some(lat), Some(lon)) = (airport.lat, airport.lon) {
                                    text(format!("Coords: {:.4}, {:.4}", lat, lon)).size(12).color(style::palette::TEXT_SECONDARY)
                                } else {
                                    text("No coordinates").size(12).color(style::palette::TEXT_SECONDARY)
                                },
                                
                                // Link back to Scenery Pack health if enabled
                                if self.map_filters.show_health_scores {
                                    if let Some(parent_pack) = self.packs.iter().find(|p| p.airports.iter().any(|a| &a.id == airport_id)) {
                                        let health = parent_pack.calculate_health_score();
                                        let (health_color, health_label) = match health {
                                            90..=100 => (style::palette::ACCENT_GREEN, "EXCELLENT"),
                                            70..=89 => (style::palette::ACCENT_BLUE, "STABLE"),
                                            40..=69 => (style::palette::ACCENT_ORANGE, "NEEDS ATTENTION"),
                                            _ => (style::palette::ACCENT_RED, "CRITICAL"),
                                        };

                                         Element::from(
                                             column![
                                                 iced::widget::vertical_space().height(10),
                                                 text("Parent Pack Health").size(10).color(style::palette::TEXT_SECONDARY),
                                                 row![
                                                     text(format!("{}%", health)).size(18).color(health_color),
                                                     text(health_label).size(10).color(health_color),
                                                 ].spacing(8).align_y(iced::Alignment::Center),
                                                     text(&parent_pack.name).size(11).color(style::palette::TEXT_SECONDARY),
                                             ].spacing(2)
                                         )
                                     } else {
                                         Element::from(iced::widget::Space::with_height(0.0))
                                     }
                                 } else {
                                     Element::from(iced::widget::Space::with_height(0.0))
                                 },
                            ].spacing(10)
                        } else {
                            column![text(format!("Airport {} (Loading...)", airport_id))]
                        }
                    } else if let Some(idx) = self.selected_flight {
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
                            let health = pack.calculate_health_score();
                            let conflicts = self.find_pack_conflicts(&pack.name);
                                                        let (health_color, health_label) = match health {
                                 90..=100 => (style::palette::ACCENT_GREEN, "EXCELLENT"),
                                 70..=89 => (style::palette::ACCENT_BLUE, "STABLE"),
                                 40..=69 => (style::palette::ACCENT_ORANGE, "NEEDS ATTENTION"),
                                 _ => (style::palette::ACCENT_RED, "CRITICAL"),
                             };
 
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
                                    .collect::<Vec<_>>()
                                )
                                .spacing(5)
                                .wrap();
                                r.into()
                            } else {
                                text("No tags").size(12).color(style::palette::TEXT_SECONDARY).into()
                            };

                            let conflict_ui: Element<'_, Message> = if !conflicts.is_empty() {
                                column![
                                    text("Potential Conflicts Detected")
                                        .size(14)
                                        .color(style::palette::ACCENT_ORANGE),
                                    column(conflicts.iter().map(|c| {
                                        text(format!("• Overlaps with: {}", c))
                                            .size(11)
                                            .color(style::palette::TEXT_SECONDARY)
                                            .into()
                                    })) 
                                ]
                                .spacing(5)
                                .into()
                            } else {
                                text("Geographically isolated (No conflicts)").size(11).color(style::palette::TEXT_SECONDARY).into()
                            };

                            let recommendation = match (pack.category.clone(), health, !conflicts.is_empty()) {
                                (_, 0..=40, _) => "Check if this pack contains actual scenery data or library files.",
                                (x_adox_core::scenery::SceneryCategory::OrthoBase, _, true) => "Ensure this is below all Libraries and Overlay scenery.",
                                (x_adox_core::scenery::SceneryCategory::Library, _, _) => "Libraries are safe anyhere, but usually go near the bottom.",
                                (_, _, true) => "Conflict detected. Move this pack HIGHER if it should override the overlapping scenery.",
                                _ => "Load order looks optimal for this category.",
                            };

                            column![
                                text(&pack.name).size(20).color(style::palette::TEXT_PRIMARY).width(Length::Fill),
                                 row![
                                     if self.map_filters.show_health_scores {
                                         Element::from(
                                             column![
                                                 text("Health Score").size(10).color(style::palette::TEXT_SECONDARY),
                                                 text(format!("{}%", health)).size(24).color(health_color).font(iced::Font::DEFAULT),
                                                 text(health_label).size(10).color(health_color),
                                             ].spacing(2)
                                         )
                                     } else {
                                         Element::from(iced::widget::Space::with_width(0.0))
                                     },
                                     iced::widget::horizontal_space().width(Length::Fixed(40.0)),
                                     column![
                                         text("Category").size(10).color(style::palette::TEXT_SECONDARY),
                                          text(format!("{:?}", pack.category)).size(14).color(style::palette::TEXT_PRIMARY).width(Length::Fill),
                                     ].spacing(2),
                                 ].align_y(iced::Alignment::Center),
                                
                                container(column![
                                    text("RECOMMENDATION").size(10).color(style::palette::TEXT_SECONDARY),
                                    text(recommendation).size(12).color(style::palette::TEXT_PRIMARY),
                                ].spacing(5))
                                .padding(10)
                                .width(Length::Fill)
                                .style(|_| container::Style {
                                    background: Some(iced::Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))),
                                    border: iced::Border {
                                        radius: 5.0.into(),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                }),

                                column![
                                    text("Content").size(14),
                                    {
                                        let el: Element<'_, Message> = if !pack.airports.is_empty() {
                                            text(format!("• {} Airports detected", pack.airports.len())).size(12).color(style::palette::TEXT_SECONDARY).into()
                                        } else {
                                            text("No airports detected").size(12).color(style::palette::TEXT_SECONDARY).into()
                                        };
                                        el
                                    },
                                    {
                                        let el: Element<'_, Message> = if !pack.tiles.is_empty() {
                                            text(format!("• {} Coverage tiles", pack.tiles.len())).size(12).color(style::palette::TEXT_SECONDARY).into()
                                        } else {
                                            text("No coverage tiles detected").size(12).color(style::palette::TEXT_SECONDARY).into()
                                        };
                                        el
                                    },
                                    {
                                        let el: Element<'_, Message> = if (pack.category == x_adox_core::scenery::SceneryCategory::RegionalOverlay || 
                                           pack.category == x_adox_core::scenery::SceneryCategory::OrthoBase) && 
                                           pack.tiles.is_empty() {
                                                text("⚠️ No tiles detected! Check if 'Earth nav data' is nested correctly.")
                                                    .size(11)
                                                    .color(style::palette::ACCENT_ORANGE)
                                                    .into()
                                        } else {
                                            iced::widget::Space::with_height(0.0).into()
                                        };
                                        el
                                    },
                                ].spacing(5),

                                if !pack.airports.is_empty() && pack.airports.len() <= 25 {
                                    let el: Element<'_, Message> = column![
                                        text("Included Airports").size(12).color(style::palette::TEXT_SECONDARY),
                                        scrollable(column(pack.airports.iter().map(|a| {
                                            text(format!("• {} - {}", a.id, a.name)).size(11).into()
                                        })).spacing(2))
                                        .height(Length::Fixed(100.0))
                                    ].spacing(5).into();
                                    el
                                } else {
                                    iced::widget::Space::with_height(0.0).into()
                                },

                                conflict_ui,
                                
                                column![
                                    text("Tags").size(14),
                                    tags_ui,
                                ].spacing(10),

                                row![
                                    text_input("New tag...", &self.new_tag_input)
                                        .on_input(Message::NewTagChanged)
                                        .size(12)
                                        .padding(8)
                                        .style(style::text_input_primary),
                                    button(text("+").size(14))
                                        .on_press(Message::AddTag(pack.name.clone(), self.new_tag_input.clone()))
                                        .style(style::button_primary)
                                        .padding([8, 12]),
                                ].spacing(5),
                            ]
                            .spacing(15)
                            .padding([10, 0])
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

    fn find_pack_conflicts(&self, pack_name: &str) -> Vec<String> {
        let target_pack = match self.packs.iter().find(|p| p.name == pack_name) {
            Some(p) => p,
            None => return Vec::new(),
        };

        if target_pack.tiles.is_empty() {
            return Vec::new();
        }

        let mut conflicts = Vec::new();
        for other in self.packs.iter() {
            if other.name == pack_name || other.status != SceneryPackType::Active {
                continue;
            }

            // Simple intersection check
            for tile in &target_pack.tiles {
                if other.tiles.contains(tile) {
                    conflicts.push(other.name.clone());
                    break;
                }
            }
        }
        conflicts
    }

    fn resort_scenery(&mut self) {
        let context = x_adox_bitnet::PredictContext {
            region_focus: self.region_focus.clone(),
            ..Default::default()
        };

        // Use the unified discovery-aware sorter
        let packs = Arc::make_mut(&mut self.packs);
        x_adox_core::scenery::sorter::sort_packs(packs, Some(&self.heuristics_model), &context);
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
            Tab::Settings => (&self.icon_settings, style::palette::ACCENT_PURPLE), // Violet for settings
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
                    iced::widget::Space::new(Length::Fill, Length::Fixed(23.0)),
                    container(iced::widget::Space::new(
                        Length::Fixed(4.0),
                        Length::Fixed(32.0)
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
        let _hovered = self.hovered_scenery.clone();
        let overrides = self.heuristics_model.config.overrides.clone();

        // Icons needed for cards
        let icons = (
            self.icon_pin.clone(),
            self.icon_pin_outline.clone(),
            self.icon_arrow_up.clone(),
            self.icon_arrow_down.clone(),
            self.icon_grip.clone(),
            self.icon_basket.clone(),
            self.icon_paste.clone(),
            self.icon_trash.clone(),
        );

        let drag_id = self.drag_context.as_ref().map(|ctx| ctx.source_index);
        let hover_id = self.drag_context.as_ref().and_then(|ctx| ctx.hover_target_index);
        let is_dragging = self.drag_context.is_some();

        let current_search_match = self.scenery_search_index
            .and_then(|idx| self.scenery_search_matches.get(idx).cloned());

        let bucket = self.scenery_bucket.clone();

        let list_container = scrollable(lazy(
            (packs, selected, overrides, drag_id, hover_id, current_search_match, bucket),
            move |(packs, selected, overrides, drag_id, hover_id, current_search_match, bucket)| {
                let mut items = Vec::new();

                for (idx, pack) in packs.iter().enumerate() {
                    // Pre-item gap
                    if is_dragging {
                        items.push(Self::view_drop_gap(idx, *hover_id == Some(idx)));
                    }

                    let is_being_dragged = *drag_id == Some(idx);
                    let is_search_match = *current_search_match == Some(idx);
                    let is_in_bucket = bucket.contains(&pack.name);

                    items.push(Self::render_scenery_card(
                        pack,
                        selected.as_ref() == Some(&pack.name),
                        overrides.contains_key(&pack.name),
                        idx,
                        is_being_dragged,
                        is_search_match,
                        is_in_bucket,
                        !bucket.is_empty(),
                        icons.clone(),
                    ));
                }

                // Final gap at the very bottom
                if is_dragging {
                    items.push(Self::view_drop_gap(packs.len(), *hover_id == Some(packs.len())));
                }

                Element::from(column(items).spacing(2)) // Tighter spacing because gaps add padding
            },
        ))
        .id(self.scenery_scroll_id.clone());

        let is_basket_open = self.show_scenery_basket;
        let basket_count = self.scenery_bucket.len();
        
        let bucket_indicator = button(
            container(
                row![
                    svg(self.icon_basket.clone())
                        .width(Length::Fixed(14.0))
                        .height(Length::Fixed(14.0)),
                    text(if is_basket_open {
                        format!("Hide Basket ({})", basket_count)
                    } else {
                        format!("Show Basket ({})", basket_count)
                    })
                    .size(12)
                    .color(if basket_count == 0 && !is_basket_open {
                        style::palette::TEXT_SECONDARY
                    } else {
                        style::palette::TEXT_PRIMARY
                    }),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center)
            )
            .padding([0, 10])
        )
        .on_press(Message::ToggleBucket)
        .style(if is_basket_open {
            style::button_primary
        } else {
            style::button_neumorphic
        })
        .padding([4, 8]);

        let main_view = column![
            row![
                text("Scenery Library").size(24).width(Length::Fill),
                {
                    let count = self.heuristics_model.config.overrides.len();
                    let mut btn = button(
                        Row::<Message, Theme, Renderer>::new()
                            .push(
                                svg(self.icon_pin.clone())
                                    .width(14)
                                    .height(14)
                                    .style(move |_, _| svg::Style {
                                        color: Some(if count > 0 {
                                            style::palette::ACCENT_RED
                                        } else {
                                            style::palette::TEXT_SECONDARY
                                        }),
                                    }),
                            )
                            .push(
                                text(format!("Clear All Pins ({})", count))
                                    .size(12)
                                    .color(if count > 0 {
                                        style::palette::TEXT_PRIMARY
                                    } else {
                                        style::palette::TEXT_SECONDARY
                                    }),
                            )
                            .spacing(8)
                            .align_y(iced::Alignment::Center),
                    );

                    if count > 0 {
                        btn = btn.on_press(Message::ClearAllPins);
                    }

                    btn.style(style::button_secondary)
                        .padding([6, 12])
                },
                {
                    let count = self.count_disabled_scenery();
                    let mut btn = button(
                        Row::<Message, Theme, Renderer>::new()
                            .push(
                                svg(self.icon_scenery.clone())
                                    .width(14)
                                    .height(14)
                                    .style(move |_, _| svg::Style {
                                        color: Some(if count > 0 {
                                            Color::WHITE
                                        } else {
                                            style::palette::TEXT_SECONDARY
                                        }),
                                    }),
                            )
                            .push(
                                text(format!("Enable All Scenery ({})", count))
                                    .size(12)
                                    .color(if count > 0 {
                                        Color::WHITE
                                    } else {
                                        style::palette::TEXT_SECONDARY
                                    }),
                            )
                            .spacing(8)
                            .align_y(iced::Alignment::Center),
                    );

                    if count > 0 {
                        btn = btn.on_press(Message::EnableAllScenery);
                    }

                    btn.style(if count > 0 {
                        style::button_enable_all
                    } else {
                        style::button_secondary
                    })
                    .padding([6, 12])
                }
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center)
            .padding(10),
            row![
                text_input("Search scenery...", &self.scenery_search_query)
                    .on_input(Message::ScenerySearchChanged)
                    .on_submit(Message::ScenerySearchSubmit)
                    .padding(8)
                    .width(Length::FillPortion(2)),
                bucket_indicator,
                row![
                    button(text("<").size(12))
                        .on_press(Message::ScenerySearchPrev)
                        .style(style::button_secondary)
                        .padding([6, 10]),
                    container(
                        text(if self.scenery_search_matches.is_empty() {
                            "0 / 0".to_string()
                        } else {
                            format!(
                                "{} / {}",
                                self.scenery_search_index.unwrap_or(0) + 1,
                                self.scenery_search_matches.len()
                            )
                        })
                        .size(12)
                        .color(style::palette::TEXT_SECONDARY)
                    )
                    .padding([0, 10]),
                    button(text(">").size(12))
                        .on_press(Message::ScenerySearchNext)
                        .style(style::button_secondary)
                        .padding([6, 10]),
                ]
                .spacing(5)
                .align_y(iced::Alignment::Center)
                .width(Length::FillPortion(1)),
            ]
            .spacing(10)
            .padding(Padding {
                top: 0.0,
                right: 10.0,
                bottom: 10.0,
                left: 10.0,
            }),
            list_container
        ]
        .spacing(10);

        main_view.into()
    }

    fn view_scenery_basket(&self) -> Element<'_, Message> {
        let bucket = self.scenery_bucket.clone();
        let selected = self.selected_basket_items.clone();

        let mut content = Column::new().spacing(10);
        
        let _is_dragging = self.basket_drag_origin.is_some();

        let header = container(
            row![
                mouse_area(
                    container(
                        row![
                            svg(self.icon_basket.clone()).width(20).height(20).style(|_, _| svg::Style { color: Some(style::palette::ACCENT_BLUE) }),
                            text("Scenery Basket").size(18),
                        ].spacing(8).align_y(iced::Alignment::Center)
                    ).width(Length::Fill)
                )
                .on_press(Message::BasketDragStart)
                .interaction(mouse::Interaction::Grab),
                row![
                    text("Auto-pin").size(10),
                    checkbox("", self.autopin_enabled).size(12).on_toggle(Message::ToggleAutopin),
                ].spacing(5).align_y(iced::Alignment::Center),
                button(text("Clear").size(10))
                    .on_press(Message::ClearBucket)
                    .style(style::button_ghost)
            ].align_y(iced::Alignment::Center)
        ).padding(Padding {
            top: 0.0,
            right: 0.0,
            bottom: 10.0,
            left: 0.0,
        });

        content = content.push(header);

        let items_list: Element<'_, Message> = if bucket.is_empty() {
            container(text("Drop scenery here...").color(style::palette::TEXT_SECONDARY))
                .width(Length::Fill)
                .height(Length::Fixed(150.0))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        } else {
            let mut list = Column::new().spacing(5);
            for item in bucket {
                let is_selected = selected.contains(&item);
                let name_clone = item.clone();
                let name_clone2 = item.clone();
                list = list.push(
                    row![
                        checkbox("", is_selected).on_toggle(move |_| Message::ToggleBasketSelection(name_clone.clone())),
                        text(item).size(12).width(Length::Fill),
                        button(
                            svg(self.icon_grip.clone())
                                .width(Length::Fixed(14.0))
                                .height(Length::Fixed(14.0))
                        )
                        .style(style::button_neumorphic)
                        .on_press(Message::DragBucketStart(Some(name_clone2))),
                    ].spacing(8).align_y(iced::Alignment::Center)
                );
            }
            container(scrollable(list).height(Length::Fill))
                .height(Length::Fill)
                .into()
        };

        let mut bottom_actions = Column::new().spacing(10);
        if !selected.is_empty() {
             bottom_actions = bottom_actions.push(
                button(
                    row![
                        svg(self.icon_grip.clone()).width(14).height(14).style(|_, _| svg::Style { color: Some(Color::WHITE) }),
                        text(format!("Drag Selected ({})", selected.len())).size(12)
                    ].spacing(8).align_y(iced::Alignment::Center)
                )
                .on_press(Message::DragBucketStart(None))
                .style(style::button_primary)
                .width(Length::Fill)
            );
        }

        let basket_content = mouse_area(
            container(
                column![
                    content,
                    container(items_list).height(Length::Fill),
                    bottom_actions
                ].spacing(15)
            )
            .padding(15)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(style::container_sidebar)
        )
        .on_enter(Message::DragEnterBasket)
        .on_exit(Message::DragLeaveBasket);

        let main_basket = container(
            stack![
                basket_content,
                // Top edge
                mouse_area(container(iced::widget::horizontal_space()).width(Length::Fill).height(5))
                    .on_press(Message::BasketResizeStart(ResizeEdge::Top))
                    .interaction(mouse::Interaction::ResizingVertically),
                // Bottom edge
                container(
                    mouse_area(container(iced::widget::horizontal_space()).width(Length::Fill).height(5))
                        .on_press(Message::BasketResizeStart(ResizeEdge::Bottom))
                        .interaction(mouse::Interaction::ResizingVertically)
                ).height(Length::Fill).align_y(iced::alignment::Vertical::Bottom),
                // Left edge
                mouse_area(container(iced::widget::horizontal_space()).width(5).height(Length::Fill))
                    .on_press(Message::BasketResizeStart(ResizeEdge::Left))
                    .interaction(mouse::Interaction::ResizingHorizontally),
                // Right edge
                container(
                    mouse_area(container(iced::widget::horizontal_space()).width(5).height(Length::Fill))
                        .on_press(Message::BasketResizeStart(ResizeEdge::Right))
                        .interaction(mouse::Interaction::ResizingHorizontally)
                ).width(Length::Fill).align_x(iced::alignment::Horizontal::Right),
                // Corners
                // Top Left
                mouse_area(container(iced::widget::horizontal_space()).width(10).height(10))
                    .on_press(Message::BasketResizeStart(ResizeEdge::TopLeft))
                    .interaction(mouse::Interaction::ResizingDiagonallyDown),
                // Top Right
                container(
                    mouse_area(container(iced::widget::horizontal_space()).width(10).height(10))
                        .on_press(Message::BasketResizeStart(ResizeEdge::TopRight))
                        .interaction(mouse::Interaction::ResizingDiagonallyUp)
                ).width(Length::Fill).align_x(iced::alignment::Horizontal::Right),
                // Bottom Left
                container(
                    mouse_area(container(iced::widget::horizontal_space()).width(10).height(10))
                        .on_press(Message::BasketResizeStart(ResizeEdge::BottomLeft))
                        .interaction(mouse::Interaction::ResizingDiagonallyUp)
                ).height(Length::Fill).align_y(iced::alignment::Vertical::Bottom),
                // Bottom Right
                container(
                    mouse_area(container(iced::widget::horizontal_space()).width(10).height(10))
                        .on_press(Message::BasketResizeStart(ResizeEdge::BottomRight))
                        .interaction(mouse::Interaction::ResizingDiagonallyDown)
                ).width(Length::Fill).height(Length::Fill).align_x(iced::alignment::Horizontal::Right).align_y(iced::alignment::Vertical::Bottom),
            ]
        )
        .width(Length::Fixed(self.basket_size.x))
        .height(Length::Fixed(self.basket_size.y));

        main_basket.into()
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
            button(text("Clear Overrides").size(14))
                .on_press(Message::ClearOverrides)
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

        // 1. Backup & Restore Section
        let backup_section: Element<'_, Message> = container(
            column![
                text("Backup & Restore").size(18),
                text("Export your profiles, scenery overrides, and sorting rules to a single file.")
                    .size(12)
                    .color(style::palette::TEXT_SECONDARY),
                row![
                    button(text("Export Config (.xback)").size(14))
                        .on_press(Message::BackupUserData)
                        .style(style::button_primary)
                        .padding([10, 20]),
                    button(text("Import Config").size(14))
                        .on_press(Message::RestoreUserData)
                        .style(style::button_secondary)
                        .padding([10, 20]),
                ]
                .spacing(10)
            ]
            .spacing(10),
        )
        .padding(20)
        .style(style::container_card)
        .width(Length::Fill)
        .into();

        // 2. Exclusions Section
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
                svg(self.icon_plugins.clone()).width(Length::Fixed(16.0)),
                text("Add Exclusion Folder")
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::AddExclusion)
        .padding(10)
        .style(style::button_primary);

        // 3. Map Filter Section
        let mut filter_content = Column::<'_, Message, Theme, Renderer>::new().spacing(5);

        filter_content = filter_content.push(
            button(
                row![text(if self.show_map_filter_settings {
                    "Map Filter v"
                } else {
                    "Map Filter >"
                })
                .size(18)]
                .spacing(10)
                .align_y(iced::Alignment::Center),
            )
            .on_press(Message::ToggleMapFilterSettings)
            .style(style::button_secondary)
            .padding(0),
        );

        if self.show_map_filter_settings {
            let filter_row = |label: &str, filter_type: MapFilterType, active: bool| {
                checkbox(label, active)
                    .on_toggle(move |_| Message::ToggleMapFilter(filter_type))
                    .size(18)
                    .text_size(14)
            };

            filter_content = filter_content.push(
                container(
                    column![
                        text("Airports & Landmarks")
                            .size(14)
                            .color(style::palette::ACCENT_MAGENTA),
                        filter_row(
                            "Custom Airports",
                            MapFilterType::CustomAirports,
                            self.map_filters.show_custom_airports
                        ),
                        filter_row(
                            "Enhancements (Small)",
                            MapFilterType::Enhancements,
                            self.map_filters.show_enhancements
                        ),
                        filter_row(
                            "Global Airports",
                            MapFilterType::GlobalAirports,
                            self.map_filters.show_global_airports
                        ),
                        iced::widget::vertical_space().height(5),
                        text("Terrain & Regions")
                            .size(14)
                            .color(style::palette::ACCENT_MAGENTA),
                        filter_row(
                            "Show Ortho Coverage",
                            MapFilterType::OrthoCoverage,
                            self.map_filters.show_ortho_coverage
                        ),
                        filter_row(
                            "Ortho Markers (Dot)",
                            MapFilterType::OrthoMarkers,
                            self.map_filters.show_ortho_markers
                        ),
                        filter_row(
                            "Regional Overlays",
                            MapFilterType::RegionalOverlays,
                            self.map_filters.show_regional_overlays
                        ),
                        iced::widget::vertical_space().height(5),
                        text("Utilities")
                            .size(14)
                            .color(style::palette::ACCENT_MAGENTA),
                        filter_row(
                            "Flight Paths",
                            MapFilterType::FlightPaths,
                            self.map_filters.show_flight_paths
                        ),
                        filter_row(
                             "Scenery Health Scores",
                             MapFilterType::HealthScores,
                             self.map_filters.show_health_scores
                         ),
                    ]
                    .spacing(8),
                )
                .padding(Padding {
                    top: 10.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 20.0,
                })
                .style(style::container_card)
            );
        }

        // Final Assembly
        scrollable(
            column![
                row![
                    button(text("Back").size(12))
                        .on_press(Message::SwitchTab(Tab::Scenery))
                        .style(style::button_secondary)
                        .padding([5, 10]),
                    title
                ]
                .spacing(20)
                .align_y(iced::Alignment::Center),
                
                backup_section,

                iced::widget::horizontal_rule(1.0),

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
                .width(Length::Fill),

                container(filter_content)
                    .padding(20)
                    .style(style::container_card)
                    .width(Length::Fill)
            ]
            .spacing(20)
            .padding(20),
        )
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
            let issues_list = column(self.log_issues.iter().enumerate().map(|(idx, issue)| {
                let is_selected = self.selected_log_issues.contains(&idx);
                row![
                    iced::widget::checkbox("", is_selected)
                        .on_toggle(move |val| Message::ToggleLogIssue(idx, val))
                        .size(18),
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
                ]
                .spacing(15)
                .align_y(iced::Alignment::Center)
                .into()
            }))
            .spacing(10);

            let all_selected = self.selected_log_issues.len() == self.log_issues.len();
            
            column![
                row![
                    iced::widget::checkbox("Select All", all_selected)
                        .on_toggle(Message::ToggleAllLogIssues)
                        .size(18),
                    text(format!(
                        "Found {} missing resources.",
                        self.log_issues.len()
                    ))
                    .color(style::palette::ACCENT_RED),
                ]
                .spacing(20)
                .align_y(iced::Alignment::Center),
                issues_list,
                row![
                    button("Re-scan Log")
                        .on_press(Message::CheckLogIssues)
                        .style(style::button_secondary),
                    button(text(format!("Export Report ({})", self.selected_log_issues.len())))
                        .on_press(Message::ExportLogIssues)
                        .style(style::button_primary),
                ]
                .spacing(10)
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
        index: usize,
        is_dragging_this: bool,
        is_search_match: bool,
        is_in_bucket: bool,
        can_paste_mode: bool,
        icons: (
            iced::widget::svg::Handle,
            iced::widget::svg::Handle,
            iced::widget::svg::Handle,
            iced::widget::svg::Handle,
            iced::widget::svg::Handle, // grip
            iced::widget::svg::Handle, // bucket
            iced::widget::svg::Handle, // paste
            iced::widget::svg::Handle, // trash
        ),
    ) -> Element<'static, Message> {
        let is_active = pack.status == SceneryPackType::Active;
        let (
            icon_pin,
            icon_pin_outline,
            icon_arrow_up,
            icon_arrow_down,
            icon_grip,
            icon_basket,
            icon_paste,
            icon_trash,
        ) = icons;

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
            x_adox_core::scenery::SceneryCategory::CustomAirport 
            | x_adox_core::scenery::SceneryCategory::OrbxAirport => style::palette::ACCENT_ORANGE,
            x_adox_core::scenery::SceneryCategory::Library => style::palette::ACCENT_BLUE,
            _ => style::palette::TEXT_SECONDARY,
        };

        let cat_display = match pack.category {
            x_adox_core::scenery::SceneryCategory::CustomAirport 
            | x_adox_core::scenery::SceneryCategory::OrbxAirport => "AIRPORT",
            x_adox_core::scenery::SceneryCategory::Library => "LIB",
            x_adox_core::scenery::SceneryCategory::Mesh 
            | x_adox_core::scenery::SceneryCategory::SpecificMesh => "MESH",
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

        let grip = button(
            svg(icon_grip)
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0))
        )
        .style(style::button_neumorphic)
        .on_press(Message::DragStart {
            index,
            name: pack.name.clone(),
        });

        let basket_btn = tooltip(
            button(
                svg(icon_basket)
                    .width(Length::Fixed(14.0))
                    .height(Length::Fixed(14.0))
            )
            .on_press(Message::ToggleBucketItem(pack.name.clone()))
            .style(if is_in_bucket { style::button_primary } else { style::button_neumorphic })
            .padding(4),
            "Add/Remove from Basket",
            tooltip::Position::Top,
        );

        let paste_btn: Element<'_, Message> = if can_paste_mode {
            button(
                svg(icon_paste)
                    .width(Length::Fixed(14.0))
                    .height(Length::Fixed(14.0))
                    .style(move |_, _| svg::Style {
                        color: Some(style::palette::ACCENT_GREEN),
                    }),
            )
            .on_press(Message::DropBucketAt(index))
            .style(style::button_ghost)
            .padding(4)
            .into()
        } else {
            iced::widget::Space::new(Length::Fixed(0.0), Length::Fixed(0.0)).into()
        };

        let delete_card_btn = button(
            svg(icon_trash)
                .width(Length::Fixed(14.0))
                .height(Length::Fixed(14.0))
                .style(move |_, _| svg::Style {
                    color: Some(style::palette::ACCENT_RED),
                }),
        )
        .on_press(Message::DeleteAddonDirect(pack.path.clone(), Tab::Scenery))
        .style(style::button_neumorphic)
        .padding(4);

        let content_row = row![
            delete_card_btn,
            basket_btn,
            grip,
            status_dot,
            info_col,
            tags_row,
            type_tag,
            move_controls,
            pin_btn,
            paste_btn,
            action_btn
        ]
        .spacing(15)
        .align_y(iced::Alignment::Center);

        let card: Element<'static, Message> = button(content_row)
            .on_press(Message::SelectScenery(pack.name.clone()))
            .style(move |theme, status| {
                let mut base = style::button_card(theme, status);
                if is_selected {
                    base.border.color = style::palette::ACCENT_BLUE;
                    base.border.width = 1.0;
                }
                if is_search_match {
                    base.border.color = Color::from_rgb(0.9, 0.9, 0.0); // Yellow highlight
                    base.border.width = 2.0;
                }
                base
            })
            .padding(15)
            .height(Length::Fixed(75.0))
            .width(Length::Fill)
            .into();

        if is_dragging_this {
            container(card)
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.5))),
                    ..Default::default()
                })
                .into()
        } else {
            card
        }
    }

    fn view_drop_gap(index: usize, is_active: bool) -> Element<'static, Message> {
        use iced::widget::{horizontal_rule, mouse_area, rule};

        let height = if is_active { 12.0 } else { 4.0 };
        let color = if is_active {
            style::palette::ACCENT_BLUE
        } else {
            Color::TRANSPARENT
        };

        mouse_area(
            container(horizontal_rule(2).style(move |_| rule::Style {
                color,
                width: 2,
                radius: 0.0.into(),
                fill_mode: rule::FillMode::Full,
            }))
            .height(Length::Fixed(height))
            .width(Length::Fill)
            .padding(if is_active { Padding::new(4.0) } else { Padding::new(0.0) })
            .style(if is_active {
                |t: &Theme| style::container_drop_gap_active(t)
            } else {
                |_: &Theme| container::Style::default()
            }),
        )
        .on_enter(Message::DragHover(index))
        .on_exit(Message::DragLeaveHover)
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
                self.icon_trash.clone(),
            ),
            move |(addons, selected_path, show_delete_confirm, active_tab, label, icon_trash)| {
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
                                style::button_primary
                            } else {
                                style::button_ghost
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
                                    button(
                                        svg(icon_trash.clone())
                                            .width(14)
                                            .height(14)
                                            .style(|_, _| svg::Style { color: Some(style::palette::ACCENT_RED) })
                                    )
                                    .on_press(Message::DeleteAddonDirect(path.clone(), if is_plugins { Tab::Plugins } else { Tab::CSLs }))
                                    .style(style::button_ghost)
                                    .padding(4),
                                ]
                                .spacing(5)
                                .width(Length::Fill)
                                .into()
                            } else {
                                button(
                                    row![
                                        text(addon.name.clone()).size(14).width(Length::Fill),
                                        text(type_label.clone())
                                            .size(12)
                                            .color(style::palette::TEXT_SECONDARY),
                                        button(
                                            svg(icon_trash.clone())
                                                .width(14)
                                                .height(14)
                                                .style(|_, _| svg::Style { color: Some(style::palette::ACCENT_RED) })
                                        )
                                        .on_press(Message::DeleteAddonDirect(path.clone(), Tab::Aircraft))
                                        .style(style::button_ghost)
                                        .padding(4),
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

        use iced::widget::lazy;

        let tree_content = if self.aircraft_tree.is_none() {
            Element::from(text("Loading aircraft...").size(14))
        } else if self.use_smart_view && self.smart_groups.is_empty() {
            Element::from(text("No aircraft found.").size(14))
        } else {
            let use_smart = self.use_smart_view;
            let tree = self.aircraft_tree.clone();
            let selected = self.selected_aircraft.clone();
            let smart_groups = self.smart_groups.clone();
            let smart_model_groups = self.smart_model_groups.clone();
            let icon_trash = self.icon_trash.clone();

            scrollable(lazy(
                (
                    use_smart,
                    tree,
                    selected,
                    self.smart_view_expanded.clone(),
                    smart_groups,
                    smart_model_groups,
                    icon_trash,
                ),
                move |(use_smart, tree, selected, expanded, smart_groups, smart_model_groups, icon_trash)| {
                    let items: Vec<Element<'_, Message, Theme, Renderer>> = if *use_smart {
                        Self::collect_smart_nodes(
                            smart_groups,
                            smart_model_groups,
                            selected,
                            expanded,
                            icon_trash.clone(),
                        )
                    } else {
                        match tree {
                            Some(t) => Self::collect_tree_nodes(t, 0, selected, icon_trash.clone()),
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
        icon_trash: svg::Handle,
    ) -> Vec<Element<'static, Message>> {
        let mut result = vec![Self::render_aircraft_row(node, depth, selected_aircraft, icon_trash.clone())];

        // Collect children if expanded
        if node.is_expanded {
            for child in &node.children {
                result.extend(Self::collect_tree_nodes(
                    child,
                    depth + 1,
                    selected_aircraft,
                    icon_trash.clone(),
                ));
            }
        }

        result
    }

    fn render_aircraft_row(
        node: &AircraftNode,
        depth: usize,
        selected_aircraft: &Option<std::path::PathBuf>,
        icon_trash: svg::Handle,
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

        let is_selected = if let Some(sel_path) = selected_aircraft {
            sel_path == &node.path && !node.path.as_os_str().is_empty()
        } else {
            false
        };

        let style = if is_selected {
            style::button_primary
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
                    .on_press(Message::ToggleAircraftFolder(path.clone()))
                    .padding([4, 8])
                    .style(style::button_ghost),
                button(text(display_name.clone()).size(14).color(label_color))
                    .on_press(if !node.path.as_os_str().is_empty() {
                        Message::SelectAircraft(path_for_select)
                    } else {
                        Message::ToggleAircraftFolder(path)
                    })
                    .style(style)
                    .padding([4, 8])
                    .width(Length::Fill)
            ]
            .spacing(5)
            .width(Length::Fill)
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
                .on_press(Message::SelectAircraft(path.clone()))
                .style(style)
                .padding([4, 8])
                .width(Length::Fill),
                button(
                    svg(icon_trash)
                        .width(14)
                        .height(14)
                        .style(|_, _| svg::Style { color: Some(style::palette::ACCENT_RED) })
                )
                .on_press(Message::DeleteAddonDirect(path.clone(), Tab::Aircraft))
                .style(style::button_ghost)
                .padding(4),
            ]
            .spacing(10)
            .width(Length::Fill)
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
        smart_groups: &std::collections::BTreeMap<String, Vec<AircraftNode>>,
        smart_model_groups: &std::collections::BTreeMap<
            String,
            std::collections::BTreeMap<String, Vec<AircraftNode>>,
        >,
        selected_aircraft: &Option<std::path::PathBuf>,
        expanded_smart: &std::collections::BTreeSet<String>,
        icon_trash: svg::Handle,
    ) -> Vec<Element<'static, Message>> {
        let mut result = Vec::new();

        for (tag, aircraft) in smart_groups {
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
                if let Some(model_groups) = smart_model_groups.get(tag) {
                    for (model, acs) in model_groups {
                        let model_id = format!("model:{}:{}", tag, model);
                        let model_expanded = expanded_smart.contains(&model_id);

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
                            for ac in acs {
                                let mut ac = ac.clone();
                                ac.acf_file = None;
                                result.push(Self::render_aircraft_row(&ac, 2, selected_aircraft, icon_trash.clone()));
                            }
                        }
                    }
                }
            } else {
                for ac in aircraft {
                    result.push(Self::render_aircraft_row(ac, 1, selected_aircraft, icon_trash.clone()));
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

    fn update_smart_view_cache(&mut self) {
        use std::collections::BTreeMap;
        let Some(tree) = &self.aircraft_tree else {
            self.smart_groups.clear();
            self.smart_model_groups.clear();
            return;
        };

        let all_aircraft = Self::flatten_aircraft_tree(tree);
        let mut groups: BTreeMap<String, Vec<AircraftNode>> = BTreeMap::new();
        for ac in all_aircraft {
            let mut tags = ac.tags.clone();
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

        let mut smart_model_groups = BTreeMap::new();
        for (tag, aircraft) in &groups {
            if MANUFACTURERS.contains(&tag.as_str()) {
                let mut model_groups: BTreeMap<String, Vec<AircraftNode>> = BTreeMap::new();
                for ac in aircraft {
                    let raw_model = ac
                        .acf_file
                        .as_ref()
                        .map(|f| f.strip_suffix(".acf").unwrap_or(f).to_string())
                        .unwrap_or_else(|| ac.name.clone());
                    let model = Self::normalize_model_name(&raw_model);
                    model_groups.entry(model).or_default().push(ac.clone());
                }
                smart_model_groups.insert(tag.clone(), model_groups);
            }
        }

        self.smart_groups = groups;
        self.smart_model_groups = smart_model_groups;
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
        Some(x_adox_core::get_config_root().join("icon_overrides.json"))
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

    fn get_scan_config_path(&self) -> Option<PathBuf> {
        let root = x_adox_core::get_scoped_config_root(self.xplane_root.as_ref()?);
        Some(root.join("scan_config.json"))
    }

    fn load_scan_config(&mut self) {
        let mut loaded = false;
        if let Some(path) = self.get_scan_config_path() {
            if let Ok(file) = std::fs::File::open(&path) {
                let reader = std::io::BufReader::new(file);
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
                    println!("Loaded {} excluded paths from scoped config", self.scan_exclusions.len());
                    loaded = true;
                }
            }
        }

        // Fallback to global config if scoped config failed or didn't exist
        if !loaded {
            let global_path = x_adox_core::get_config_root().join("scan_config.json");
            if global_path.exists() {
                if let Ok(file) = std::fs::File::open(global_path) {
                    let reader = std::io::BufReader::new(file);
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
                        println!("Loaded {} excluded paths from global config (fallback)", self.scan_exclusions.len());
                    }
                }
            } else {
                // Reset to empty if no config found at all
                self.scan_exclusions = Vec::new();
                self.scan_inclusions = Vec::new();
            }
        }
    }

    fn save_scan_config(&self) {
        if let Some(path) = self.get_scan_config_path() {
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
        Some(x_adox_core::get_config_root().join("app_config.json"))
    }

    fn load_app_config() -> (Option<PathBuf>, Vec<CompanionApp>, MapFilters) {
        if let Some(path) = Self::get_app_config_path() {
            if let Ok(file) = std::fs::File::open(path) {
                let reader = std::io::BufReader::new(file);

                #[derive(serde::Deserialize)]
                struct AppConfig {
                    selected_xplane_path: Option<PathBuf>,
                    companion_apps: Option<Vec<CompanionApp>>,
                    map_filters: Option<MapFilters>,
                }

                if let Ok(config) = serde_json::from_reader::<_, AppConfig>(reader) {
                    return (
                        config.selected_xplane_path,
                        config.companion_apps.unwrap_or_default(),
                        config.map_filters.unwrap_or_default(),
                    );
                }
            }
        }
        (None, Vec::new(), MapFilters::default())
    }

    fn initialize_heuristics(xplane_root: &Path) -> BitNetModel {
        let scoped_root = x_adox_core::get_scoped_config_root(xplane_root);
        let scoped_path = scoped_root.join("heuristics.json");

        if !scoped_path.exists() {
            // Check for global heuristics to migrate
            if let Some(proj_dirs) = directories::ProjectDirs::from("org", "x-adox", "x-adox") {
                let global_path = proj_dirs.config_dir().join("heuristics.json");
                if global_path.exists() {
                    println!("[Migration] Migrating global heuristics to scoped path: {:?}", scoped_path);
                    if let Ok(content) = std::fs::read_to_string(&global_path) {
                        let _ = std::fs::write(&scoped_path, content);
                    }
                }
            }
        }

        BitNetModel::at_path(scoped_path)
    }

    fn save_app_config(&self) {
        if let Some(path) = Self::get_app_config_path() {
            if let Ok(file) = std::fs::File::create(path) {
                #[derive(serde::Serialize)]
                struct AppConfig<'a> {
                    selected_xplane_path: Option<&'a PathBuf>,
                    companion_apps: &'a Vec<CompanionApp>,
                    map_filters: &'a MapFilters,
                }
                let config = AppConfig {
                    selected_xplane_path: self.xplane_root.as_ref(),
                    companion_apps: &self.companion_apps,
                    map_filters: &self.map_filters,
                };
                let writer = std::io::BufWriter::new(file);
                let _ = serde_json::to_writer_pretty(writer, &config);
            }
        }
    }

    fn filtered_logbook_indices(&self) -> Vec<usize> {
        self.logbook
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                if !self.logbook_filter_aircraft.is_empty() {
                    let filter = self.logbook_filter_aircraft.to_lowercase();
                    if !entry.tail_number.to_lowercase().contains(&filter)
                        && !entry.aircraft_type.to_lowercase().contains(&filter)
                    {
                        return false;
                    }
                }
                if self.logbook_filter_circular && entry.dep_airport != entry.arr_airport {
                    return false;
                }
                if let Ok(min) = self.logbook_filter_duration_min.parse::<f64>() {
                    if entry.total_duration < min {
                        return false;
                    }
                }
                if let Ok(max) = self.logbook_filter_duration_max.parse::<f64>() {
                    if entry.total_duration > max {
                        return false;
                    }
                }
                true
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    fn spawn_companion_app(&mut self, app: &CompanionApp) {
        let path = &app.path;
        let parent_dir = path.parent().unwrap_or(path);

        let mut command = if path.extension().map_or(false, |ext| ext == "sh") {
            let mut cmd = std::process::Command::new("sh");
            cmd.arg("-c").arg(format!("\"{}\"", path.display()));
            cmd
        } else {
            std::process::Command::new(path)
        };

        command.current_dir(parent_dir);

        // Escape AppImage sandbox if necessary
        command.env_remove("LD_LIBRARY_PATH");
        command.env_remove("APPDIR");
        command.env_remove("APPIMAGE");

        if let Err(e) = command.spawn() {
            self.status = format!("Failed to launch {}: {}", app.name, e);
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

    let mut cache = x_adox_core::cache::DiscoveryCache::load(Some(&root));
    let aircraft = DiscoveryManager::scan_aircraft(&root, &mut cache, &exclusions);
    let _ = cache.save(Some(&root));

    Ok(Arc::new(aircraft))
}

fn load_plugins(root: Option<PathBuf>) -> Result<Arc<Vec<DiscoveredAddon>>, String> {
    let root = root.ok_or("X-Plane root not found")?;
    let mut cache = x_adox_core::cache::DiscoveryCache::load(Some(&root));
    let plugins = DiscoveryManager::scan_plugins(&root, &mut cache);
    let _ = cache.save(Some(&root));
    Ok(Arc::new(plugins))
}

fn load_csls(root: Option<PathBuf>) -> Result<Arc<Vec<DiscoveredAddon>>, String> {
    let root = root.ok_or("X-Plane root not found")?;
    let mut cache = x_adox_core::cache::DiscoveryCache::load(Some(&root));
    let csls = DiscoveryManager::scan_csls(&root, &mut cache);
    let _ = cache.save(Some(&root));
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


fn save_scenery_packs(
    root: PathBuf,
    packs: Vec<SceneryPack>,
    model: BitNetModel,
) -> Result<(), String> {
    let xpm = XPlaneManager::new(&root).map_err(|e| e.to_string())?;
    let mut sm = SceneryManager::new(xpm.get_scenery_packs_path());
    sm.packs = packs;
    sm.save(Some(&model)).map_err(|e| e.to_string())?;
    Ok(())
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

    // Load BitNet model for tagging (heuristics are GLOBAL)
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
    model: BitNetModel,
    context: x_adox_bitnet::PredictContext,
    on_progress: impl FnMut(f32) + Send + 'static,
) -> Result<String, String> {
    let res = tokio::task::spawn_blocking(move || {
        extract_archive_task(root, zip_path, tab, dest_override, model, context, on_progress)
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
    model: BitNetModel,
    context: x_adox_bitnet::PredictContext,
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
        
        // Auto-sort every time a new pack is installed
        sm.sort(Some(&model), &context);
        sm.save(Some(&model)).map_err(|e| e.to_string())?;
    }

    Ok(top_folder)
}

fn extract_7z_task(
    root: Option<PathBuf>,
    archive_path: PathBuf,
    tab: Tab,
    dest_override: Option<PathBuf>,
    model: BitNetModel,
    context: x_adox_bitnet::PredictContext,
    mut on_progress: impl FnMut(f32) + Send + 'static,
) -> Result<String, String> {
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

    // Signal start of extraction
    on_progress(5.0);

    // Extract using sevenz-rust2
    sevenz_rust2::decompress_file(&archive_path, &dest_dir)
        .map_err(|e| format!("Failed to extract 7z: {}", e))?;

    // Signal extraction complete
    on_progress(90.0);

    // Determine the top-level folder name from the archive filename
    let top_folder = archive_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    // Special handling for Scenery: add to scenery_packs.ini
    if matches!(tab, Tab::Scenery) {
        let xpm = XPlaneManager::new(&root).map_err(|e| e.to_string())?;
        let mut sm = SceneryManager::new(xpm.get_scenery_packs_path());
        sm.load().map_err(|e| e.to_string())?;

        // Auto-sort every time a new pack is installed
        sm.sort(Some(&model), &context);
        sm.save(Some(&model)).map_err(|e| e.to_string())?;
    }

    on_progress(100.0);
    Ok(top_folder)
}

fn extract_archive_task(
    root: Option<PathBuf>,
    archive_path: PathBuf,
    tab: Tab,
    dest_override: Option<PathBuf>,
    model: BitNetModel,
    context: x_adox_bitnet::PredictContext,
    on_progress: impl FnMut(f32) + Send + 'static,
) -> Result<String, String> {
    let extension = archive_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "zip" => extract_zip_task(root, archive_path, tab, dest_override, model, context, on_progress),
        "7z" => extract_7z_task(root, archive_path, tab, dest_override, model, context, on_progress),
        _ => Err(format!("Unsupported archive format: .{}", extension)),
    }
}

fn delete_addon(root: Option<PathBuf>, path: PathBuf, tab: Tab) -> Result<(), String> {
    let root = root.ok_or("X-Plane root not found")?;

    let addon_type = match tab {
        Tab::Scenery => ManagementAddonType::Scenery,
        Tab::Aircraft => ManagementAddonType::Aircraft,
        Tab::Plugins => ManagementAddonType::Plugins,
        Tab::CSLs => ManagementAddonType::CSLs,
        _ => return Err("Invalid tab for deletion".to_string()),
    };

    ModManager::delete_addon(&root, &path, addon_type)
}

async fn load_logbook_data(
    path: PathBuf,
) -> Result<Vec<x_adox_core::logbook::LogbookEntry>, String> {
    if !path.exists() {
        return Err(format!("Logbook not found: {}", path.display()));
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

async fn pick_archive(label: &str) -> Option<PathBuf> {
    log::debug!("Opening archive picker for {}", label);
    rfd::AsyncFileDialog::new()
        .set_title(&format!("Select {} Package (.zip, .7z)", label))
        .add_filter("Archives", &["zip", "7z"])
        .pick_file()
        .await
        .map(|f| {
            let p = f.path().to_path_buf();
            log::info!("Selected archive for {}: {:?}", label, p);
            p
        })
}

async fn pick_folder(title: &str, start_dir: Option<PathBuf>) -> Option<PathBuf> {
    log::debug!("Opening folder picker: {}", title);
    let mut dialog = rfd::AsyncFileDialog::new().set_title(title);
    if let Some(path) = start_dir {
        if path.exists() {
            dialog = dialog.set_directory(&path);
        }
    }
    dialog.pick_folder().await.map(|f| {
        let p = f.path().to_path_buf();
        log::info!("Selected folder for {}: {:?}", title, p);
        p
    })
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
    current_packs: Arc<Vec<SceneryPack>>,
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
    
    // Instead of loading from disk (which has old order), use the current GUI order
    sm.packs = (*current_packs).clone();
    
    let (packs, report) = sm.simulate_sort(&model, &context);
    Ok((Arc::new(packs), report))
}

fn save_packs_task(
    root: Option<PathBuf>,
    packs: Arc<Vec<SceneryPack>>,
    model: BitNetModel,
) -> Result<(), String> {
    let root = root.ok_or("X-Plane root not found")?;
    let xpm = XPlaneManager::new(&root).map_err(|e| e.to_string())?;
    let ini_path = xpm.get_scenery_packs_path();

    // Backups are now handled automatically by sm.save() in x-adox-core

    let mut sm = SceneryManager::new(ini_path);
    sm.packs = packs.as_ref().clone();
    sm.save(Some(&model)).map_err(|e| e.to_string())
}

async fn apply_profile_task(
    root: Option<PathBuf>,
    profile: Profile,
    mut model: x_adox_bitnet::BitNetModel,
) -> Result<(), String> {
    let root = root.ok_or("X-Plane root not found")?;

    // 1. Scenery Enablement & Order
    let scenery_ini_path = root.join("Custom Scenery").join("scenery_packs.ini");
    let mut manager =
        x_adox_core::scenery::SceneryManager::new(scenery_ini_path);
    manager.load().map_err(|e| e.to_string())?;

    // Apply enablement
    manager.set_bulk_states(&profile.scenery_states);

    // Apply profile-specific pins to the model before sorting
    let overrides = profile
        .scenery_overrides
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect::<std::collections::BTreeMap<_, _>>();
    model.apply_overrides(overrides);

    // Sort and Save
    manager.sort(
        Some(&model),
        &x_adox_bitnet::PredictContext::default(),
    );
    manager.save(Some(&model)).map_err(|e| e.to_string())?;

    // 2. Plugins & Aircraft require discovering current paths to correctly toggle
    let mut cache = x_adox_core::cache::DiscoveryCache::load(Some(&root));

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

    let _ = cache.save(Some(&root));
    Ok(())
}

fn export_log_issues_task(issues: Arc<Vec<x_adox_core::LogIssue>>) -> Result<PathBuf, String> {
    use std::fs::File;
    use std::io::Write;

    let initial_location = directories::UserDirs::new()
        .and_then(|u| u.document_dir().map(|d| d.to_path_buf()))
        .or_else(|| directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let path = rfd::FileDialog::new()
        .add_filter("CSV File", &["csv"])
        .add_filter("Text File", &["txt"])
        .set_file_name("x_plane_missing_resources.csv")
        .set_directory(&initial_location)
        .save_file()
        .ok_or("Export cancelled".to_string())?;

    let is_csv = path.extension().map_or(false, |ext| ext == "csv");

    let mut content = String::new();
    if is_csv {
        content.push_str("Resource Path,Referenced From,Potential Library\n");
        for issue in issues.iter() {
            let lib = issue.potential_library.as_deref().unwrap_or("None");
            // Simple CSV escaping: wrap in quotes and escape internal quotes
            let res = issue.resource_path.replace('"', "\"\"");
            let pkg = issue.package_path.replace('"', "\"\"");
            let lib_esc = lib.replace('"', "\"\"");
            content.push_str(&format!("\"{}\",\"{}\",\"{}\"\n", res, pkg, lib_esc));
        }
    } else {
        content.push_str("X-Plane Missing Resources Report\n");
        content.push_str("==============================\n\n");
        for issue in issues.iter() {
            content.push_str(&format!("Missing Resource: {}\n", issue.resource_path));
            content.push_str(&format!("Referenced from:  {}\n", issue.package_path));
            if let Some(lib) = &issue.potential_library {
                content.push_str(&format!("Potential Library: {}\n", lib));
            }
            content.push_str("------------------------------\n");
        }
        content.push_str(&format!("\nTotal Issues: {}\n", issues.len()));
    }

    let mut file = File::create(&path).map_err(|e| format!("Failed to create file: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(path)
}
