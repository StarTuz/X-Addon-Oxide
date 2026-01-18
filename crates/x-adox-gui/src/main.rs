use iced::widget::{
    button, checkbox, column, container, image, progress_bar, responsive, row, scrollable, stack,
    svg, text, text_editor, Column,
};
use iced::{Background, Border, Color, Element, Length, Task, Theme};
use std::path::PathBuf;
use x_adox_bitnet::BitNetModel;
use x_adox_core::discovery::{AddonType, DiscoveredAddon, DiscoveryManager};
use x_adox_core::management::ModManager;
use x_adox_core::scenery::{SceneryManager, SceneryPack, SceneryPackType};
use x_adox_core::XPlaneManager;

mod map;
mod style;
use map::{MapView, TileManager};

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
    Heuristics,
    Issues,
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
    InstallProgress(f32),
    SmartSort,

    // Heuristics
    OpenHeuristicsEditor,
    HeuristicsAction(text_editor::Action),
    SaveHeuristics,
    ImportHeuristics,
    ExportHeuristics,
    ResetHeuristics,
    HeuristicsImported(String),

    // Issues
    LogIssuesLoaded(Result<Vec<x_adox_core::LogIssue>, String>),
    CheckLogIssues,
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
    install_progress: Option<f32>,
    // Heuristics
    heuristics_model: BitNetModel,
    heuristics_json: text_editor::Content,
    heuristics_error: Option<String>,

    // Issues
    log_issues: Vec<x_adox_core::LogIssue>,
    icon_warning: svg::Handle,
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
            install_progress: None,
            heuristics_model: BitNetModel::new().unwrap_or_default(),
            heuristics_json: text_editor::Content::new(),
            heuristics_error: None,
            log_issues: Vec::new(),
            icon_warning: svg::Handle::from_memory(
                include_bytes!("../assets/icons/warning.svg").to_vec(),
            ),
        };

        let tasks = if let Some(r) = root {
            let r1 = r.clone();
            let r2 = r.clone();
            let r3 = r.clone();
            let r4 = r.clone();
            let r5 = r.clone();
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
                Task::perform(
                    async move { load_log_issues(Some(r5)) },
                    Message::LogIssuesLoaded,
                ),
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
                };
                Task::none()
            }
            Message::LogIssuesLoaded(result) => {
                match result {
                    Ok(issues) => {
                        self.log_issues = issues;
                        if !self.log_issues.is_empty() {
                            self.status =
                                format!("Found {} issues in Log.txt", self.log_issues.len());
                        }
                    }
                    Err(e) => self.status = format!("Log analysis error: {}", e),
                }
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
                    Tab::Heuristics | Tab::Issues => None,
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
                        Tab::Heuristics | Tab::Issues => None,
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
                let json =
                    serde_json::to_string_pretty(&self.heuristics_model.config).unwrap_or_default();
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
                        self.heuristics_model.config = config;
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
                    let json = serde_json::to_string_pretty(&self.heuristics_model.config)
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
            Message::SmartSort => {
                let root = self.xplane_root.clone();
                return Task::perform(
                    async move {
                        let root = root.ok_or("X-Plane root not found")?;
                        let xpm = XPlaneManager::new(&root).map_err(|e| e.to_string())?;
                        let ini_path = xpm.get_scenery_packs_path();

                        // --- Safety Backup Logic (Timestamped) ---
                        if ini_path.exists() {
                            let parent = ini_path.parent().unwrap_or(&ini_path);
                            // 1. Rotate existing bak1_TIMESTAMP -> bak2_TIMESTAMP
                            if let Ok(entries) = std::fs::read_dir(parent) {
                                for entry in entries.flatten() {
                                    let path = entry.path();
                                    let filename = path.file_name().unwrap().to_string_lossy();
                                    if filename.starts_with("scenery_packs.ini.bak1_") {
                                        // Found a bak1, rotate it to bak2 preserving timestamp
                                        let new_name = filename.replace(".bak1_", ".bak2_");
                                        let new_path = parent.join(new_name);
                                        let _ = std::fs::rename(&path, &new_path);
                                    }
                                }
                            }

                            // 2. Create new bak1_CURRENT_TIMESTAMP
                            let timestamp =
                                chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
                            let bak1_name = format!("scenery_packs.ini.bak1_{}", timestamp);
                            let bak1_path = parent.join(bak1_name);
                            let _ = std::fs::copy(&ini_path, &bak1_path);
                        }

                        let mut sm = SceneryManager::new(ini_path);
                        sm.load().map_err(|e| e.to_string())?;
                        sm.sort();
                        sm.save().map_err(|e| e.to_string())?;
                        Ok::<(), String>(())
                    },
                    |res| match res {
                        Ok(_) => Message::Refresh,
                        Err(e) => Message::SceneryLoaded(Err(e)),
                    },
                );
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
                self.sidebar_button("Issues", Tab::Issues),
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
            Tab::Heuristics => self.view_heuristics_editor(),
            Tab::Issues => self.view_issues(),
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
        if let Some(btn) = smart_sort_btn {
            actions = actions.push(btn);

            // Add Edit Sort button next to Smart Sort
            let edit_sort_btn = button(text("Edit Sort").size(12))
                .on_press(Message::OpenHeuristicsEditor)
                .style(style::button_premium_glow)
                .padding([6, 12]);
            actions = actions.push(edit_sort_btn);
        }

        container(
            column![
                // Top Bar
                row![
                    actions,
                    iced::widget::horizontal_space(),
                    text(path_text)
                        .size(12)
                        .color(style::palette::TEXT_SECONDARY),
                    button(text("Set").size(12).color(Color::WHITE))
                        .on_press(Message::SelectFolder)
                        .style(style::button_secondary)
                        .padding([4, 8]),
                ]
                .spacing(20)
                .align_y(iced::Alignment::Center),
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

    fn sidebar_button(&self, label: &'static str, tab: Tab) -> Element<'_, Message> {
        let is_active = self.active_tab == tab;

        let (icon_handle, active_color) = match tab {
            Tab::Aircraft => (&self.icon_aircraft, Color::WHITE),
            Tab::Scenery => (&self.icon_scenery, Color::from_rgb(0.4, 0.8, 0.4)), // Green
            Tab::Plugins => (&self.icon_plugins, Color::from_rgb(0.4, 0.6, 1.0)), // Blue
            Tab::CSLs => (&self.icon_csls, Color::from_rgb(1.0, 0.6, 0.2)),       // Orange
            Tab::Heuristics => (&self.refresh_icon, Color::from_rgb(0.8, 0.8, 0.8)), // Gray
            Tab::Issues => {
                if self.log_issues.is_empty() {
                    (&self.icon_warning, Color::from_rgb(0.6, 0.6, 0.6)) // Dim gray
                } else {
                    (&self.icon_warning, Color::from_rgb(1.0, 0.2, 0.2)) // Red alert
                }
            }
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
            let glow_icon = svg(icon_handle.clone())
                .width(Length::Fixed(54.0)) // Slightly larger for "bloom"
                .height(Length::Fixed(54.0))
                .style(move |_theme, _status| svg::Style {
                    color: Some(Color::from_rgba(
                        active_color.r,
                        active_color.g,
                        active_color.b,
                        0.2,
                    )),
                });

            container(stack![
                container(glow_icon)
                    .width(Length::Fixed(48.0))
                    .height(Length::Fixed(48.0))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
                container(icon)
                    .width(Length::Fixed(48.0))
                    .height(Length::Fixed(48.0))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            ])
        } else {
            container(icon)
                .width(Length::Fixed(48.0))
                .height(Length::Fixed(48.0))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
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

        let list_container = scrollable(list).id(self.scenery_scroll_id.clone());

        column![
            row![text("Scenery Library").size(24).width(Length::Fill)]
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

    fn view_issues(&self) -> Element<'_, Message> {
        let title = text("Detected Log Issues")
            .size(32)
            .color(style::palette::ACCENT_ORANGE);

        if self.log_issues.is_empty() {
            return container(
                column![
                    title,
                    text("No issues detected in Log.txt").size(20),
                    text("X-Plane seems to be finding all resources correctly.").size(16),
                    button("Re-scan Log")
                        .padding([10, 20])
                        .style(style::button_primary)
                        .on_press(Message::CheckLogIssues),
                ]
                .spacing(20),
            )
            .padding(40)
            .center_x(Length::Fill)
            .into();
        }

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
                        row![]
                    },
                ]
                .spacing(8),
            )
            .padding(15)
            .style(style::container_card)
            .width(Length::Fill)
            .into()
        }))
        .spacing(15);

        container(
            column![
                row![
                    title,
                    iced::widget::horizontal_space(),
                    button("Re-scan Log")
                        .padding([8, 16])
                        .style(style::button_primary)
                        .on_press(Message::CheckLogIssues),
                ]
                .align_y(iced::Alignment::Center),
                text(format!(
                    "Found {} missing resources in your last X-Plane session.",
                    self.log_issues.len()
                ))
                .size(16)
                .color(style::palette::TEXT_SECONDARY),
                scrollable(issues_list).height(Length::Fill),
            ]
            .spacing(20),
        )
        .padding(30)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(style::container_main_content)
        .into()
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

async fn install_addon(
    root: Option<PathBuf>,
    zip_path: PathBuf,
    tab: Tab,
    dest_override: Option<PathBuf>,
    mut on_progress: impl FnMut(f32),
) -> Result<String, String> {
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
            Tab::Heuristics | Tab::Issues => {
                return Err("Cannot install to Heuristics or Issues tab".to_string())
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
    let total_files = archive.len();
    for i in 0..total_files {
        {
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

                // Optimization: use a buffer to copy instead of read_to_end
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Failed to extract file: {}", e))?;
            }
        }

        // Update progress
        let progress = ((i + 1) as f32 / total_files as f32) * 100.0;
        on_progress(progress);
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
            Tab::CSLs | Tab::Heuristics | Tab::Issues => unreachable!(), // Handled above or not applicable
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

fn load_log_issues(root: Option<PathBuf>) -> Result<Vec<x_adox_core::LogIssue>, String> {
    let root = root.ok_or("X-Plane root not found")?;
    let xpm = XPlaneManager::new(&root).map_err(|e| e.to_string())?;
    xpm.check_log().map_err(|e| e.to_string())
}
