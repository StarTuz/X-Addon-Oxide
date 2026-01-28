use crate::Message;
use iced::advanced::{self, layout, renderer, widget, Layout, Widget};
use iced::widget::image;
use iced::{mouse, Border, Color, Element, Event, Length, Radians, Rectangle};
use lru::LruCache;
use std::num::NonZeroUsize;

use std::sync::{Arc, Mutex};
use x_adox_core::scenery::{SceneryPack, SceneryPackType};

// --- Slippy Map / Mercator Math ---
pub const TILE_SIZE: f64 = 256.0;

pub fn lon_to_x(lon: f64, zoom: f64) -> f64 {
    ((lon + 180.0) / 360.0) * 2.0f64.powf(zoom) * TILE_SIZE
}

pub fn lat_to_y(lat: f64, zoom: f64) -> f64 {
    let lat_rad = lat.to_radians();
    (1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0
        * 2.0f64.powf(zoom)
        * TILE_SIZE
}

pub fn x_to_lon(x: f64, zoom: f64) -> f64 {
    (x / (TILE_SIZE * 2.0f64.powf(zoom))) * 360.0 - 180.0
}

pub fn y_to_lat(y: f64, zoom: f64) -> f64 {
    let n = std::f64::consts::PI - 2.0 * std::f64::consts::PI * y / (TILE_SIZE * 2.0f64.powf(zoom));
    (0.5 * (n.exp() - (-n).exp())).atan().to_degrees()
}

// --- Tile Management ---
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoords {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl TileCoords {
    pub fn url(&self) -> String {
        format!(
            "https://tile.openstreetmap.org/{}/{}/{}.png",
            self.z, self.x, self.y
        )
    }
}

pub struct TileManager {
    tiles: Arc<Mutex<LruCache<TileCoords, image::Handle>>>,
    pending: Arc<Mutex<std::collections::HashSet<TileCoords>>>,
}

impl TileManager {
    pub fn new() -> Self {
        Self {
            tiles: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(300).unwrap()))),
            pending: Arc::new(Mutex::new(std::collections::HashSet::new())),
        }
    }

    pub fn get_tile(&self, coords: TileCoords) -> Option<image::Handle> {
        let mut tiles = self.tiles.lock().unwrap();
        tiles.get(&coords).cloned()
    }

    pub fn request_tile(&self, coords: TileCoords) {
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

        // Persistent Disk Cache Path
        let cache_path = directories::ProjectDirs::from("com", "x-adox", "X-Addon-Oxide")
            .map(|dirs| {
                dirs.cache_dir()
                    .join("tiles")
                    .join(coords.z.to_string())
                    .join(coords.x.to_string())
                    .join(format!("{}.png", coords.y))
            })
            .unwrap_or_else(|| {
                std::path::PathBuf::from("tiles")
                    .join(coords.z.to_string())
                    .join(coords.x.to_string())
                    .join(format!("{}.png", coords.y))
            });

        std::thread::spawn(move || {
            // 1. Check disk cache first
            if cache_path.exists() {
                if let Ok(bytes) = std::fs::read(&cache_path) {
                    let handle = image::Handle::from_bytes(bytes);
                    let mut tiles = tiles_arc.lock().unwrap();
                    tiles.put(coords, handle);
                    let mut pending = pending_arc.lock().unwrap();
                    pending.remove(&coords);
                    return;
                }
            }

            // 2. Fetch from network if not on disk
            let resp = ureq::get(&coords.url())
                .set("User-Agent", "X-Addon-Oxide/0.1.0")
                .timeout(std::time::Duration::from_secs(10))
                .call();

            match resp {
                Ok(response) => {
                    let mut bytes = Vec::new();
                    if let Ok(_) =
                        std::io::Read::read_to_end(&mut response.into_reader(), &mut bytes)
                    {
                        let handle = image::Handle::from_bytes(bytes.clone());
                        let mut tiles = tiles_arc.lock().unwrap();
                        tiles.put(coords, handle);

                        // 3. Save to disk cache
                        if let Some(parent) = cache_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = std::fs::write(&cache_path, bytes);
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
pub struct MapView<'a> {
    pub packs: &'a [SceneryPack],
    pub selected_scenery: Option<&'a String>,
    pub hovered_scenery: Option<&'a String>,
    pub hovered_airport_id: Option<&'a String>,
    pub tile_manager: &'a TileManager,
    pub zoom: f64,          // Fractional zoom (e.g., 2.5)
    pub center: (f64, f64), // (Lat, Lon)
    pub airports: &'a std::collections::HashMap<String, x_adox_core::apt_dat::Airport>,
    pub selected_flight: Option<&'a x_adox_core::logbook::LogbookEntry>,
    pub filters: &'a crate::MapFilters,
}

impl<'a> MapView<'a> {
    pub fn is_pack_visible(&self, pack: &SceneryPack) -> bool {
        use x_adox_core::scenery::SceneryCategory;

        // 1. Technical Category Identification
        let is_ortho = pack.category == SceneryCategory::OrthoBase;
        let is_library = pack.category == SceneryCategory::Library;

        let is_regional = matches!(
            pack.category,
            SceneryCategory::RegionalOverlay
                | SceneryCategory::RegionalFluff
                | SceneryCategory::OrbxAirport
                | SceneryCategory::AutoOrthoOverlay
                | SceneryCategory::LowImpactOverlay
        );

        let is_airport_enhancement = matches!(
            pack.category,
            SceneryCategory::AirportOverlay | SceneryCategory::Landmark
        );

        let is_global_apt = pack.category == SceneryCategory::GlobalAirport;
        let is_custom_apt =
            !pack.airports.is_empty() && pack.category != SceneryCategory::OrbxAirport;

        // 2. Filter Application
        if is_global_apt {
            self.filters.show_global_airports
        } else if is_custom_apt {
            self.filters.show_custom_airports
        } else if is_ortho {
            self.filters.show_ortho_markers
        } else if is_library {
            false
        } else if is_regional {
            // Regional packs (SimHeaven, Orbx, AO, Low-Impact) always follow the Regional filter
            self.filters.show_regional_overlays
        } else if is_airport_enhancement {
            // Airport-specific overlays use the "Enhancements" filter
            self.filters.show_enhancements
        } else {
            // Default: All other internal categories (Mesh, GlobalBase, Unknown) are hidden
            // unless we decide to add a toggle back for them later.
            false
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct MapState {
    is_dragging: bool,
    press_position: Option<iced::Point>,
    last_cursor: Option<iced::Point>,
    // Track values between prop updates to handle multiple events per frame
    current_center: (f64, f64), // (lat, lon)
    current_zoom: f64,
    last_prop_center: Option<(f64, f64)>,
    last_prop_zoom: Option<f64>,
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
        tree: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<MapState>();
        let bounds = layout.bounds();

        // Prefer internal state for zero-latency feedback during interactions
        let zoom = if state.last_prop_zoom.is_some() {
            state.current_zoom
        } else {
            self.zoom
        };
        let (center_lat, center_lon) = if state.last_prop_center.is_some() {
            state.current_center
        } else {
            self.center
        };

        let zoom_scale = 2.0f64.powf(zoom);

        let camera_center_x = lon_to_x(center_lon, 0.0);
        let camera_center_y = lat_to_y(center_lat, 0.0);

        let half_w = (bounds.width as f64 / 2.0) / zoom_scale;
        let half_h = (bounds.height as f64 / 2.0) / zoom_scale;

        let view_left = camera_center_x - half_w;
        let view_right = camera_center_x + half_w;
        let view_top = camera_center_y - half_h;
        let view_bottom = camera_center_y + half_h;

        let center_offset_x = (bounds.x + bounds.width / 2.0) as f64 - camera_center_x * zoom_scale;
        let center_offset_y =
            (bounds.y + bounds.height / 2.0) as f64 - camera_center_y * zoom_scale;

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
            let square_size = 6.0;
            let selected_size = 10.0;

            // --- Conflict Detection ---
            // If both Global and Custom Airports are enabled, we build a set of Custom ICAOs
            // to suppress Global dots that are redundant.
            let mut custom_icaos = std::collections::HashSet::new();
            if self.filters.show_custom_airports && self.filters.show_global_airports {
                for pack in self.packs {
                    if pack.category != x_adox_core::scenery::SceneryCategory::GlobalAirport
                        && self.is_pack_visible(pack)
                    {
                        for apt in &pack.airports {
                            custom_icaos.insert(&apt.id);
                        }
                    }
                }
            }

            for pack in self.packs.iter().rev() {
                let is_selected = self.selected_scenery == Some(&pack.name);
                let is_hovered = self.hovered_scenery == Some(&pack.name);
                let is_ortho = pack.category == x_adox_core::scenery::SceneryCategory::OrthoBase;
                let is_library = pack.category == x_adox_core::scenery::SceneryCategory::Library;

                let is_regional = matches!(
                    pack.category,
                    x_adox_core::scenery::SceneryCategory::RegionalOverlay
                        | x_adox_core::scenery::SceneryCategory::RegionalFluff
                        | x_adox_core::scenery::SceneryCategory::OrbxAirport
                        | x_adox_core::scenery::SceneryCategory::AutoOrthoOverlay
                        | x_adox_core::scenery::SceneryCategory::LowImpactOverlay
                );

                let base_color = match pack.status {
                    SceneryPackType::Active => Color::from_rgb(0.0, 1.0, 0.0), // Classic Green
                    SceneryPackType::Disabled | SceneryPackType::DuplicateHidden => {
                        Color::from_rgb(1.0, 0.2, 0.2) // keeping the brighter red for better contrast
                    }
                };

                let size = if is_selected || is_hovered {
                    selected_size
                } else {
                    square_size
                };

                // Visual Styling Determination
                let (fill_color, radius) = if is_selected || is_hovered {
                    (Color::from_rgb(1.0, 1.0, 0.0), (size / 4.0).into())
                } else if is_ortho {
                    (Color::from_rgb(0.0, 1.0, 1.0), 0.0.into()) // Cyan Sharp Square
                } else if is_library {
                    (Color::from_rgb(1.0, 0.0, 1.0), (size / 2.0).into()) // Purple Circle
                } else if is_regional {
                    (Color::from_rgb(0.5, 1.0, 0.0), (size / 2.0).into()) // Lime "Diamond" (Circle)
                } else {
                    (base_color, (size / 4.0).into()) // Standard rounded square
                };

                let half_size = size / 2.0;

                let is_mesh = pack.category == x_adox_core::scenery::SceneryCategory::Mesh
                    || pack.category == x_adox_core::scenery::SceneryCategory::SpecificMesh
                    || pack.category == x_adox_core::scenery::SceneryCategory::GlobalBase;

                let is_global =
                    pack.category == x_adox_core::scenery::SceneryCategory::GlobalAirport;
                let is_massive = is_mesh || is_regional || is_global;
                let is_visible = self.is_pack_visible(pack);
                let should_draw_dots = (is_selected && !is_massive) || is_visible;

                if should_draw_dots {
                    // Optimization: High-performance thinning for massive packs
                    let skip_step = if is_massive {
                        if zoom < 3.0 {
                            50
                        } else if zoom < 5.0 {
                            15
                        } else if zoom < 7.0 {
                            4
                        } else if zoom < 9.0 {
                            2
                        } else {
                            1
                        }
                    } else {
                        1
                    };

                    // Pre-calculate lat/lon bounds for early-exit viewport check
                    let view_lon_min = x_to_lon(view_left, 0.0) - 1.0;
                    let view_lon_max = x_to_lon(view_right, 0.0) + 1.0;
                    let view_lat_min = y_to_lat(view_bottom, 0.0) - 1.0;
                    let view_lat_max = y_to_lat(view_top, 0.0) + 1.0;

                    // Draw Tile-based dots (for ortho/mesh/enhancements)
                    if pack.airports.is_empty() {
                        for (i, &(lat, lon)) in pack.tiles.iter().enumerate() {
                            if i % skip_step != 0 {
                                continue;
                            }
                            // 1. Fast lat/lon early exit
                            let lat_f = lat as f64;
                            let lon_f = lon as f64;

                            if lat_f < view_lat_min
                                || lat_f > view_lat_max
                                || lon_f < view_lon_min
                                || lon_f > view_lon_max
                            {
                                continue;
                            }

                            // 2. Precise pixel check (optional but kept for safety at edge of bounds)
                            let wx = lon_to_x(lon_f + 0.5, 0.0);
                            let wy = lat_to_y(lat_f + 0.5, 0.0);

                            if wx < view_left
                                || wx > view_right
                                || wy < view_top
                                || wy > view_bottom
                            {
                                continue;
                            }

                            let sx = (center_offset_x + wx * zoom_scale) as f32;
                            let sy = (center_offset_y + wy * zoom_scale) as f32;

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
                                        radius,
                                    },
                                    ..Default::default()
                                },
                                fill_color,
                            );
                        }
                    }

                    // Draw Airport-based markers
                    for (i, airport) in pack.airports.iter().enumerate() {
                        // --- Conflict Resolution ---
                        // Skip global markers if a higher-priority custom pack covers it
                        if is_global && custom_icaos.contains(&airport.id) {
                            continue;
                        }

                        if i % skip_step != 0 {
                            continue;
                        }

                        // 1. Fast lat/lon early exit
                        if let (Some(lat), Some(lon)) = (airport.lat, airport.lon) {
                            if lat < view_lat_min
                                || lat > view_lat_max
                                || lon < view_lon_min
                                || lon > view_lon_max
                            {
                                continue;
                            }
                        }
                        if let (Some(px), Some(py)) = (airport.proj_x, airport.proj_y) {
                            // Quick viewport check in z0 pixels
                            let wx = px as f64 * TILE_SIZE;
                            let wy = py as f64 * TILE_SIZE;

                            if wx < view_left
                                || wx > view_right
                                || wy < view_top
                                || wy > view_bottom
                            {
                                continue;
                            }

                            let sx = (center_offset_x + wx * zoom_scale) as f32;
                            let sy = (center_offset_y + wy * zoom_scale) as f32;

                            // For Global Airports, check individual airport hover
                            let is_airport_hovered =
                                is_global && self.hovered_airport_id == Some(&airport.id);

                            // For Global Airports, ONLY highlight the hovered one. Pack selection is ignored to avoid "Yellow Ocean".
                            let highlight_this = (is_selected && !is_global) || is_airport_hovered;

                            let airport_size = if highlight_this {
                                selected_size
                            } else {
                                square_size
                            };

                            let (airport_fill_color, airport_radius) =
                                if highlight_this || (is_hovered && !is_global) {
                                    (Color::from_rgb(1.0, 1.0, 0.0), (airport_size / 4.0).into())
                                } else {
                                    (base_color, (airport_size / 4.0).into())
                                };

                            let airport_half_size = airport_size / 2.0;

                            renderer.fill_quad(
                                renderer::Quad {
                                    bounds: Rectangle {
                                        x: sx - airport_half_size,
                                        y: sy - airport_half_size,
                                        width: airport_size,
                                        height: airport_size,
                                    },
                                    border: iced::Border {
                                        color: Color::BLACK,
                                        width: 1.0,
                                        radius: airport_radius,
                                    },
                                    ..Default::default()
                                },
                                airport_fill_color,
                            );
                        }
                    }
                }

                // 2. Should we draw "ORTHO COVERAGE" rectangles?
                if is_ortho && self.filters.show_ortho_coverage {
                    let ortho_color = Color::from_rgba(0.0, 0.5, 1.0, 0.3); // Transparent Blue
                    for &(lat, lon) in &pack.tiles {
                        let wx1 = lon_to_x(lon as f64, 0.0);
                        let wy1 = lat_to_y(lat as f64 + 1.0, 0.0);
                        let wx2 = lon_to_x(lon as f64 + 1.0, 0.0);
                        let wy2 = lat_to_y(lat as f64, 0.0);

                        let sx1 = bounds.x
                            + (bounds.width / 2.0)
                            + ((wx1 - camera_center_x) * zoom_scale) as f32;
                        let sy1 = bounds.y
                            + (bounds.height / 2.0)
                            + ((wy1 - camera_center_y) * zoom_scale) as f32;
                        let sx2 = bounds.x
                            + (bounds.width / 2.0)
                            + ((wx2 - camera_center_x) * zoom_scale) as f32;
                        let sy2 = bounds.y
                            + (bounds.height / 2.0)
                            + ((wy2 - camera_center_y) * zoom_scale) as f32;

                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: Rectangle {
                                    x: sx1,
                                    y: sy1,
                                    width: sx2 - sx1,
                                    height: sy2 - sy1,
                                },
                                border: iced::Border {
                                    color: Color::from_rgba(0.0, 0.5, 1.0, 0.8),
                                    width: 1.0,
                                    radius: 0.0.into(),
                                },
                                ..Default::default()
                            },
                            ortho_color,
                        );
                    }
                }
            }
        });

        // --- Flight Path Layer ---
        if self.filters.show_flight_paths {
            if let Some(flight) = self.selected_flight {
                let dep_coords = self
                    .airports
                    .get(&flight.dep_airport)
                    .and_then(|a| a.lat.zip(a.lon));
                let arr_coords = self
                    .airports
                    .get(&flight.arr_airport)
                    .and_then(|a| a.lat.zip(a.lon));

                if let (Some((lat1, lon1)), Some((lat2, lon2))) = (dep_coords, arr_coords) {
                    renderer.with_layer(bounds, |renderer| {
                        let wx1 = lon_to_x(lon1, 0.0);
                        let wy1 = lat_to_y(lat1, 0.0);
                        let wx2 = lon_to_x(lon2, 0.0);
                        let wy2 = lat_to_y(lat2, 0.0);

                        let sx1 = bounds.x
                            + (bounds.width / 2.0)
                            + ((wx1 - camera_center_x) * zoom_scale) as f32;
                        let sy1 = bounds.y
                            + (bounds.height / 2.0)
                            + ((wy1 - camera_center_y) * zoom_scale) as f32;
                        let sx2 = bounds.x
                            + (bounds.width / 2.0)
                            + ((wx2 - camera_center_x) * zoom_scale) as f32;
                        let sy2 = bounds.y
                            + (bounds.height / 2.0)
                            + ((wy2 - camera_center_y) * zoom_scale) as f32;

                        // Draw the flight line using point interpolation
                        let dx = sx2 - sx1;
                        let dy = sy2 - sy1;
                        let distance = (dx * dx + dy * dy).sqrt();
                        let steps = (distance / 4.0).ceil().max(1.0) as usize; // Ensure at least 1 step
                        for i in 0..=steps {
                            let t = i as f32 / steps as f32;
                            let px = sx1 + dx * t;
                            let py = sy1 + dy * t;
                            renderer.fill_quad(
                                renderer::Quad {
                                    bounds: Rectangle {
                                        x: px - 1.0,
                                        y: py - 1.0,
                                        width: 2.0,
                                        height: 2.0,
                                    },
                                    ..Default::default()
                                },
                                Color::from_rgb(1.0, 0.0, 1.0), // Magenta
                            );
                        }

                        // Special case: If distance is 0 (same airport), draw a larger indicator
                        if distance < 1.0 {
                            renderer.fill_quad(
                                renderer::Quad {
                                    bounds: Rectangle {
                                        x: sx1 - 4.0,
                                        y: sy1 - 4.0,
                                        width: 8.0,
                                        height: 8.0,
                                    },
                                    border: Border {
                                        color: Color::from_rgb(1.0, 0.0, 1.0),
                                        width: 2.0,
                                        radius: 4.0.into(),
                                    },
                                    ..Default::default()
                                },
                                Color::TRANSPARENT,
                            );
                        }

                        // Markers for DEP and ARR (drawn after the line for visibility)
                        let dot_size = 8.0;
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: Rectangle {
                                    x: sx1 - 4.0,
                                    y: sy1 - 4.0,
                                    width: dot_size,
                                    height: dot_size,
                                },
                                border: Border {
                                    color: Color::BLACK,
                                    width: 1.0,
                                    radius: 4.0.into(),
                                },
                                ..Default::default()
                            },
                            Color::from_rgb(0.0, 1.0, 1.0), // Cyan DEP
                        );
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: Rectangle {
                                    x: sx2 - 4.0,
                                    y: sy2 - 4.0,
                                    width: dot_size,
                                    height: dot_size,
                                },
                                border: Border {
                                    color: Color::BLACK,
                                    width: 1.0,
                                    radius: 4.0.into(),
                                },
                                ..Default::default()
                            },
                            Color::from_rgb(1.0, 0.5, 0.0), // Orange ARR
                        );
                    });
                }
            }
        }
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
        let zoom_prop = self.zoom;
        let center_prop = self.center;

        // Initialize or sync internal state from props if props changed externally
        if state.last_prop_center != Some(center_prop) || state.last_prop_zoom != Some(zoom_prop) {
            state.current_center = center_prop;
            state.current_zoom = zoom_prop;
            state.last_prop_center = Some(center_prop);
            state.last_prop_zoom = Some(zoom_prop);
        }

        let current_zoom = state.current_zoom;
        let (center_lat, center_lon) = state.current_center;

        let camera_x = lon_to_x(center_lon, 0.0);
        let camera_y = lat_to_y(center_lat, 0.0);
        let scale = 2.0f64.powf(current_zoom);

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
                    let new_zoom = (current_zoom + d * 0.2).clamp(min_zoom, 19.0);

                    if (new_zoom - current_zoom).abs() > 0.001 {
                        let new_scale = 2.0f64.powf(new_zoom);

                        let mx = (p.x as f64) - (bounds.width as f64 / 2.0);
                        let my = (p.y as f64) - (bounds.height as f64 / 2.0);

                        let new_camera_x = camera_x + mx / scale - mx / new_scale;
                        let new_camera_y = camera_y + my / scale - my / new_scale;

                        let new_half_w = (bounds.width as f64 / 2.0) / new_scale;
                        let new_camera_x_clamped =
                            new_camera_x.clamp(new_half_w, TILE_SIZE - new_half_w);
                        let new_camera_y_clamped = new_camera_y.clamp(0.0, TILE_SIZE);

                        let new_center = (
                            y_to_lat(new_camera_y_clamped, 0.0),
                            x_to_lon(new_camera_x_clamped, 0.0),
                        );

                        // Update internal state immediately for next event in same frame
                        state.current_center = new_center;
                        state.current_zoom = new_zoom;

                        shell.publish(Message::MapZoom {
                            new_center,
                            new_zoom,
                        });
                        return advanced::graphics::core::event::Status::Captured;
                    }
                }
            }
            Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                // START DRAG if over bounds (even if over scenery)
                if cursor.is_over(bounds) {
                    if let Some(position) = cursor.position() {
                        state.is_dragging = true;
                        state.press_position = Some(position);
                        state.last_cursor = Some(position);
                        return advanced::graphics::core::event::Status::Captured;
                    }
                }
            }
            Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                let was_dragging = state.is_dragging;
                let press_pos = state.press_position;
                let release_pos = cursor.position();

                state.is_dragging = false;
                state.press_position = None;
                state.last_cursor = None;

                if was_dragging {
                    // Check if it was a "click" (minimal movement)
                    if let (Some(p1), Some(p2)) = (press_pos, release_pos) {
                        let dist = (p1.x - p2.x).hypot(p1.y - p2.y);
                        if dist < 5.0 {
                            // ACTUALLY A CLICK - Perform selection
                            if let Some(coords) = coords {
                                for pack in self.packs {
                                    // Strictly ignore packs that are filtered out
                                    if !self.is_pack_visible(pack) {
                                        continue;
                                    }

                                    if pack.airports.is_empty() {
                                        for &(lat, lon) in &pack.tiles {
                                            if (lat as f64 + 0.5 - coords.0).abs() < 0.5
                                                && (lon as f64 + 0.5 - coords.1).abs() < 0.5
                                            {
                                                shell.publish(Message::SelectScenery(
                                                    pack.name.clone(),
                                                ));
                                                return advanced::graphics::core::event::Status::Captured;
                                            }
                                        }
                                    }
                                    for airport in &pack.airports {
                                        if let (Some(lat), Some(lon), Some((wx, wy))) =
                                            (airport.lat, airport.lon, mouse_z0)
                                        {
                                            let tx = lon_to_x(lon as f64, 0.0);
                                            let ty = lat_to_y(lat as f64, 0.0);
                                            let dist_sq = (tx - wx).powi(2) + (ty - wy).powi(2);

                                            if dist_sq < (10.0 / scale).powi(2) {
                                                shell.publish(Message::SelectScenery(
                                                    pack.name.clone(),
                                                ));
                                                return advanced::graphics::core::event::Status::Captured;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
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

                        let new_center = (y_to_lat(clamped_wy, 0.0), x_to_lon(clamped_wx, 0.0));

                        // Update internal state immediately for next event (e.g. multiple moves in one frame)
                        state.current_center = new_center;

                        shell.publish(Message::MapZoom {
                            new_center,
                            new_zoom: current_zoom,
                        });
                        return advanced::graphics::core::event::Status::Captured;
                    }
                }

                if let Some(coords) = coords {
                    // --- Conflict Detection ---
                    let mut custom_icaos = std::collections::HashSet::new();
                    if self.filters.show_custom_airports && self.filters.show_global_airports {
                        for pack in self.packs {
                            if pack.category != x_adox_core::scenery::SceneryCategory::GlobalAirport
                                && self.is_pack_visible(pack)
                            {
                                for apt in &pack.airports {
                                    custom_icaos.insert(&apt.id);
                                }
                            }
                        }
                    }

                    for pack in self.packs {
                        // Ignore packs that are filtered out unless they are selected
                        if !self.is_pack_visible(pack) && self.selected_scenery != Some(&pack.name)
                        {
                            continue;
                        }

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

                        let is_global =
                            pack.category == x_adox_core::scenery::SceneryCategory::GlobalAirport;

                        for airport in &pack.airports {
                            // --- Conflict Resolution ---
                            if is_global && custom_icaos.contains(&airport.id) {
                                continue;
                            }

                            if let (Some(lat), Some(lon), Some((wx, wy))) =
                                (airport.lat, airport.lon, mouse_z0)
                            {
                                // Fast early exit: skip airports far from cursor
                                // The 10px radius at current zoom translates to roughly:
                                let hit_radius_deg = 10.0 / scale / TILE_SIZE * 360.0;
                                if (lon as f64 - coords.1).abs() > hit_radius_deg * 2.0
                                    || (lat as f64 - coords.0).abs() > hit_radius_deg * 2.0
                                {
                                    continue;
                                }

                                let tx = lon_to_x(lon as f64, 0.0);
                                let ty = lat_to_y(lat as f64, 0.0);
                                let dist_sq = (tx - wx).powi(2) + (ty - wy).powi(2);

                                // Use a 10px hit radius in screen pixels
                                if dist_sq < (10.0 / scale).powi(2) {
                                    // Always emit airport hover for Inspector Panel
                                    if self.hovered_airport_id != Some(&airport.id) {
                                        shell.publish(Message::HoverAirport(Some(
                                            airport.id.clone(),
                                        )));
                                    }

                                    // Always emit pack hover for highlights
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
                    // Clear hover state when cursor is not over any airport/tile
                    // Only emit if state is changing (prevent continuous message flood)
                    if self.hovered_scenery.is_some() {
                        shell.publish(Message::HoverScenery(None));
                    }
                    if self.hovered_airport_id.is_some() {
                        shell.publish(Message::HoverAirport(None));
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
