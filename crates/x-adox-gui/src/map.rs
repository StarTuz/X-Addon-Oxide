use crate::Message;
use iced::advanced::{self, layout, renderer, widget, Layout, Widget};
use iced::widget::image;
use iced::{mouse, Color, Element, Event, Length, Radians, Rectangle};
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

pub struct MapView<'a> {
    pub packs: &'a [SceneryPack],
    pub selected_scenery: Option<&'a String>,
    pub hovered_scenery: Option<&'a String>,
    pub tile_manager: &'a TileManager,
    pub zoom: f64,          // Fractional zoom (e.g., 2.5)
    pub center: (f64, f64), // (Lat, Lon)
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
                    SceneryPackType::Disabled | SceneryPackType::DuplicateHidden => {
                        Color::from_rgb(1.0, 0.0, 0.0)
                    }
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
