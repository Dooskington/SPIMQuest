extern crate sdl2;
extern crate time;

use std::str;
use std::path::*;
use sdl2::*;
use sdl2::video::Window;
use sdl2::render::Canvas;
use sdl2::pixels::PixelFormatEnum;
use sdl2::pixels::Color;
use sdl2::surface::*;
use sdl2::image::*;
use sdl2::event::*;
use sdl2::keyboard::*;
use sdl2::gfx::primitives::DrawRenderer;
use time::*;

pub const WINDOW_TITLE: &'static str = "SPIM Quest";
pub const WINDOW_WIDTH: u32 = 640;
pub const WINDOW_HEIGHT: u32 = 480;
pub const FIELD_OF_VIEW: f64 = 90.0;

pub const COLOR_BLACK: Color = Color {r: 0, g: 0, b: 0, a: 255};
pub const COLOR_WHITE: Color = Color {r: 255, g: 255, b: 255, a: 255};
pub const COLOR_RED: Color = Color {r: 255, g: 0, b: 0, a: 255};
pub const COLOR_GREEN: Color = Color {r: 0, g: 255, b: 0, a: 255};
pub const COLOR_BLUE: Color = Color {r: 0, g: 0, b: 255, a: 255};
pub const COLOR_MAGENTA: Color = Color {r: 255, g: 0, b: 255, a: 255};

pub const TWO_PI: f64 = 2.0 * std::f64::consts::PI;

pub struct Map {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<Option<Tile>>
}

impl Map {
    pub fn new(width: u32, height: u32, tiles: Vec<Option<Tile>>) -> Map {
        Map {
            width: width,
            height: height,
            tiles: tiles
        }
    }

    pub fn get_tile(&self, x: u32, y: u32) -> Option<Tile> {
        self.tiles[((y * self.width) + x) as usize]
    }

    pub fn load(file_path: &str) -> std::io::Result<Map> {
        let texture: Texture = Texture::load(file_path)
            .expect(&format!("Failed to load map texture {}", file_path));

        let mut tiles: Vec<Option<Tile>> = Vec::new();
        tiles.resize((texture.width * texture.height) as usize, None);

        for x in 0..texture.width {
            for y in 0..texture.height {
                let index: usize = ((y * texture.width) + x) as usize;
                let color: Color = texture.pixels[index];

                match color {
                    // Wall
                    COLOR_BLACK => {
                        tiles[index] = Some(Tile::new(x, y, 0));
                    },

                    // Debug
                    COLOR_RED => {
                        tiles[index] = Some(Tile::new(x, y, 1));
                    },
                    _ => {}
                }
            }
        }

        Ok(Map::new(texture.width, texture.height, tiles))
    }
}

#[derive(Copy, Clone)]
pub struct Tile {
    pub x: u32,
    pub y: u32,
    pub id: u32
}

impl Tile {
    pub fn new(x: u32, y: u32, id: u32) -> Tile {
        Tile {
            x: x,
            y: y,
            id: id
        }
    }
}

pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Color>
}

impl Texture {
    pub fn new(width: u32, height: u32, pixels: Vec<Color>) -> Texture {
        Texture {
            width: width,
            height: height,
            pixels: pixels
        }
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Color {
        self.pixels[((y * self.width) + x) as usize]
    }

    pub fn load(file_path: &str) -> std::io::Result<Texture> {
        let sdl_surface: Surface = Surface::from_file(Path::new(file_path))
            .expect(&format!("Failed to create surface from file path {}", file_path));

        let mut pixels: Vec<Color> = Vec::new();
        pixels.resize((sdl_surface.width() * sdl_surface.height()) as usize, COLOR_MAGENTA);

        // Read the pixels from the surface to convert into a texture
        sdl_surface.with_lock(|surface_buffer: &[u8]| {
            for x in 0..sdl_surface.width() {
                for y in 0..sdl_surface.height() {
                    // Calculate the index of this pixel
                    let index : usize =
                        (y as usize * sdl_surface.pitch() as usize) +
                        (x as usize * sdl_surface.pixel_format_enum().byte_size_per_pixel());

                    // Convert the pixel into a color
                    let color = Color {
                        r: surface_buffer[index],
                        g: surface_buffer[index + 1],
                        b: surface_buffer[index + 2],
                        a: surface_buffer[index + 3],
                    };

                    // Store the new color
                    pixels[((y * sdl_surface.width()) + x) as usize] = color;
                }
            }
        });

        Ok(Texture::new(sdl_surface.width(), sdl_surface.height(), pixels))
    }
}

pub struct RaycastHit {
    pub x: f64,
    pub y: f64,
    pub tile_x: u32,
    pub tile_y: u32,
    pub tile_side: u8,
    pub distance: f64
}

pub struct Game {
    sdl_context: Sdl,
    sdl_canvas: Canvas<Window>,
    start_time: Tm,
    map: Map,

    wall_texture: Texture,
    ceiling_texture: Texture,
    floor_texture: Texture,

    player_x: f64,
    player_y: f64,
    player_rotation: f64,
    input_left: bool,
    input_right: bool,
    input_up: bool,
    input_down: bool,
    input_strafe_left: bool,
    input_strafe_right: bool
}

impl Game {
    pub fn new() -> Game {
        let sdl_context: Sdl = ::sdl2::init().expect("Failed to initialize SDL!");
        let sdl_video: VideoSubsystem = sdl_context.video().expect("Failed to initialize video!");

        let sdl_window: Window = sdl_video.window(WINDOW_TITLE, WINDOW_WIDTH, WINDOW_HEIGHT)
            .position_centered()
            .opengl()
            .build()
            .expect("Failed to create window!");

        let sdl_canvas: Canvas<Window> = sdl_window
            .into_canvas()
            .target_texture()
            .build()
            .expect("Failed to get canvas!");

        let map = Map::load("res/maps/level1.png")
            .expect("Failed to load map!");

        Game {
            sdl_context: sdl_context,
            sdl_canvas: sdl_canvas,
            start_time: time::now(),
            map: map,
            wall_texture: Texture::load("res/wall.png").unwrap(),
            ceiling_texture: Texture::load("res/ceiling.png").unwrap(),
            floor_texture: Texture::load("res/floor.png").unwrap(),
            player_x: 3.5,
            player_y: 3.5,
            player_rotation: 0.0,
            input_left: false,
            input_right: false,
            input_up: false,
            input_down: false,
            input_strafe_left: false,
            input_strafe_right: false
        }
    }

    pub fn run(&mut self) {
        let mut last_tick_time: Tm = time::now();
        let mut render_timer: Duration = time::Duration::zero();
        let sixty_hz: Duration = time::Duration::nanoseconds(16666667); // TODO: Consider a const?

        let mut sdl_event_pump = self.sdl_context.event_pump()
            .expect("Failed to run event loop!");

        'running: loop {
            // Timing
            let current_time: Tm = time::now();
            let elapsed_time: Duration = current_time - last_tick_time;
            let delta_time: f64 = (elapsed_time.num_nanoseconds().unwrap() as f64) / 1_000_000_000_f64;
            render_timer = render_timer + elapsed_time;

            // Handle window events
            for event in sdl_event_pump.poll_iter() {
                match event {
                    Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), ..} => {
                        break 'running;
                    },

                    Event::KeyDown { keycode: Some(Keycode::Left), .. } | Event::KeyDown { keycode: Some(Keycode::A), .. } => {
                        self.input_left = true;
                    },
                    Event::KeyUp { keycode: Some(Keycode::Left), .. } | Event::KeyUp { keycode: Some(Keycode::A), .. } => {
                        self.input_left = false;
                    },
                    Event::KeyDown { keycode: Some(Keycode::Right), .. } | Event::KeyDown { keycode: Some(Keycode::D), .. } => {
                        self.input_right = true;
                    },
                    Event::KeyUp { keycode: Some(Keycode::Right), .. } | Event::KeyUp { keycode: Some(Keycode::D), .. } => {
                        self.input_right = false;
                    },
                    Event::KeyDown { keycode: Some(Keycode::Q), .. } => {
                        self.input_strafe_left = true;
                    },
                    Event::KeyUp { keycode: Some(Keycode::Q), .. } => {
                        self.input_strafe_left = false;
                    },
                    Event::KeyDown { keycode: Some(Keycode::E), .. } => {
                        self.input_strafe_right = true;
                    },
                    Event::KeyUp { keycode: Some(Keycode::E), .. } => {
                        self.input_strafe_right = false;
                    },
                    Event::KeyDown { keycode: Some(Keycode::Up), .. } | Event::KeyDown { keycode: Some(Keycode::W), .. } => {
                        self.input_up = true;
                    },
                    Event::KeyUp { keycode: Some(Keycode::Up), .. } | Event::KeyUp { keycode: Some(Keycode::W), .. } => {
                        self.input_up = false;
                    },
                    Event::KeyDown { keycode: Some(Keycode::Down), .. } | Event::KeyDown { keycode: Some(Keycode::S), .. } => {
                        self.input_down = true;
                    },
                    Event::KeyUp { keycode: Some(Keycode::Down), .. } | Event::KeyUp { keycode: Some(Keycode::S), .. } => {
                        self.input_down = false;
                    },

                    _ => {}
                }
            }

            let rotation_speed: f64 = f64::to_radians(150.0);
            let move_speed: f64 = 3.5;

            // Calculate velocity based on input
            let mut velocity_x: f64 = 0.0;
            let mut velocity_y: f64 = 0.0;

            if self.input_up {
                velocity_x += self.player_rotation.cos() * move_speed;
                velocity_y += self.player_rotation.sin() * move_speed;
            }
            if self.input_down {
                velocity_x -= self.player_rotation.cos() * move_speed;
                velocity_y -= self.player_rotation.sin() * move_speed;
            }
            if self.input_strafe_left {
                velocity_x -= f64::cos(self.player_rotation + (std::f64::consts::PI / 2.0)) * move_speed;
                velocity_y -= f64::sin(self.player_rotation + (std::f64::consts::PI / 2.0)) * move_speed;
            }
            if self.input_strafe_right {
                velocity_x += f64::cos(self.player_rotation + (std::f64::consts::PI / 2.0)) * move_speed;
                velocity_y += f64::sin(self.player_rotation + (std::f64::consts::PI / 2.0)) * move_speed;
            }
            if self.input_left {
                self.player_rotation = self.wrap_angle(self.player_rotation - (rotation_speed * delta_time));
            }
            if self.input_right {
                self.player_rotation = self.wrap_angle(self.player_rotation + (rotation_speed * delta_time));
            }

            // Apply velocity
            if (velocity_x != 0.0) || (velocity_y != 0.0) {
                let new_position_x = self.player_x + (velocity_x * delta_time);
                let new_position_y = self.player_y + (velocity_y * delta_time);

                if self.map.get_tile(new_position_x.trunc() as u32, self.player_y.trunc() as u32).is_none() {
                    self.player_x = new_position_x;
                }

                if self.map.get_tile(self.player_x.trunc() as u32, new_position_y.trunc() as u32).is_none() {
                    self.player_y = new_position_y;
                }
            }

            // TODO:
            // This may be broken
            last_tick_time = time::now();

            // Render
            if render_timer >= sixty_hz {
                self.sdl_canvas.set_draw_color(COLOR_BLACK);
                self.sdl_canvas.clear();

                self.render_world();

                self.sdl_canvas.present();
            }
        }
    }

    pub fn wrap_angle(&self, angle: f64) -> f64 {
        if angle < 0.0 {
            return angle + TWO_PI;
        }
        else if angle >= TWO_PI {
            return angle - TWO_PI;
        }

        angle
    }

    fn calculate_lighting(&self, distance: f64, light_radius: f64) -> f64 {
        ((light_radius - distance) * (1.0 / light_radius)).max(0.0).min(1.0)
    }

    fn render_world(&mut self) {
        let projection_width: u32 = WINDOW_WIDTH;
        let projection_height: u32 = WINDOW_HEIGHT;
        let projection_distance: f64 = (projection_width as f64 / 2.0) / f64::tan(FIELD_OF_VIEW.to_radians() / 2.0);
        let light_radius: f64 = 5.0;
        let origin_x: f64 = self.player_x;
        let origin_y: f64 = self.player_y;
        let rotation: f64 = self.player_rotation;
        let tile_width: f64 = 1.0;
        let tile_height: f64 = 1.0;
        let player_height: f64 = 0.5;

        // Raycasting
        for x in 0..projection_width {
            // The vertical stripe that this ray is going through
            let ray_screen_x: f64 = -(projection_width as f64) / 2.0 + x as f64;

            // The distance from the viewer to the stripe on the screen;
            let ray_view_dist = (ray_screen_x.powi(2) + projection_distance.powi(2)).sqrt();

            // Calculate the angle of the ray and cast it
            let ray_angle: f64 = (ray_screen_x / ray_view_dist).asin() + rotation;
            let intersection: RaycastHit = self.raycast(origin_x, origin_y, ray_angle);

            // Calculate the actual distance
            let intersection_distance = intersection.distance.sqrt() * (rotation - ray_angle).cos();

            let tile = self.map.get_tile(intersection.tile_x, intersection.tile_y).unwrap();

            let ref wall_texture: Texture = self.wall_texture;
            let ref ceiling_texture: Texture = self.ceiling_texture;
            let ref floor_texture: Texture = self.floor_texture;

            // Calculate the x texel of this wall strip
            let wall_texture_x: u32 = if intersection.tile_side == 0 {
                (((intersection.y - (intersection.tile_y as f64 * tile_width)) % tile_width) * (wall_texture.width - 1) as f64).round() as u32
            } else {
                (((intersection.x - (intersection.tile_x as f64 * tile_width)) % tile_width) * (wall_texture.width - 1) as f64).round() as u32
            };

            // Calculate the values for the wall strip
            let line_height: i32 = ((tile_height * projection_distance) / intersection_distance).round() as i32;
            let line_screen_start: i32 = (projection_height as i32 / 2) - (line_height / 2);
            let line_screen_end: i32 = line_screen_start + line_height;

            let wall_lighting: f64 = self.calculate_lighting(intersection_distance, light_radius);

            for y in 0..projection_height {
                // Walls
                if ((y as i32) >= line_screen_start) && ((y as i32) < line_screen_end) {
                    let line_y: i32 = y as i32 - line_screen_start;
                    let texture_y: u32 = f64::floor((line_y as f64 / line_height as f64) * (wall_texture.height - 1) as f64) as u32;

                    let mut color: Color = wall_texture.get_pixel(wall_texture_x, texture_y);
                    color.r = ((if intersection.tile_side == 0 { color.r } else { color.r / 2 }) as f64 * wall_lighting) as u8;
                    color.g = ((if intersection.tile_side == 0 { color.g } else { color.g / 2 }) as f64 * wall_lighting) as u8;
                    color.b = ((if intersection.tile_side == 0 { color.b } else { color.b / 2 }) as f64 * wall_lighting) as u8;

                    self.sdl_canvas.pixel(x as i16, y as i16, color).unwrap();
                    continue;
                }

                // Floors
                if (y as i32) >= line_screen_end {
                    let floor_row: i32 = (y as i32) - (projection_height as i32 / 2);

                    let floor_straight_distance = (player_height / floor_row as f64) * projection_distance;
                    let angle_beta_radians = rotation - ray_angle;

                    let floor_actual_distance = floor_straight_distance / angle_beta_radians.cos();

                    let mut floor_hit_x: f64 = origin_x + (floor_actual_distance * ray_angle.cos());
                    let mut floor_hit_y: f64 = origin_y + (floor_actual_distance * ray_angle.sin());

                    floor_hit_x -= floor_hit_x.floor();
                    floor_hit_y -= floor_hit_y.floor();

                    let texture_x: u32 = f64::floor(floor_hit_x * (floor_texture.width - 1) as f64) as u32;
                    let texture_y: u32 = f64::floor(floor_hit_y * (floor_texture.height - 1) as f64) as u32;

                    let floor_lighting: f64 = self.calculate_lighting(floor_straight_distance, light_radius);
                    let mut color: Color = floor_texture.get_pixel(texture_x, texture_y);;
                    color.r = (color.r as f64 * floor_lighting) as u8;
                    color.g = (color.g as f64 * floor_lighting) as u8;
                    color.b = (color.b as f64 * floor_lighting) as u8;

                    self.sdl_canvas.pixel(x as i16, y as i16, color).unwrap();
                    continue;
                }

                // Ceilings
                if (y as i32) < line_screen_start {
                    let ceiling_row: i32 = (y as i32) - (projection_height as i32 / 2);

                    let ceiling_straight_distance = (player_height / ceiling_row as f64) * projection_distance;
                    let angle_beta_radians = rotation - ray_angle;

                    let ceiling_actual_distance = ceiling_straight_distance / angle_beta_radians.cos();

                    let mut ceiling_hit_x: f64 = origin_x - (ceiling_actual_distance * ray_angle.cos());
                    let mut ceiling_hit_y: f64 = origin_y - (ceiling_actual_distance * ray_angle.sin());

                    ceiling_hit_x -= ceiling_hit_x.floor();
                    ceiling_hit_y -= ceiling_hit_y.floor();

                    let texture_x: u32 = f64::floor(ceiling_hit_x * (ceiling_texture.width - 1) as f64) as u32;
                    let texture_y: u32 = f64::floor(ceiling_hit_y * (ceiling_texture.height - 1) as f64) as u32;

                    let ceiling_lighting: f64 = self.calculate_lighting(ceiling_straight_distance.abs(), light_radius);
                    let mut color: Color = ceiling_texture.get_pixel(texture_x, texture_y);;
                    color.r = (color.r as f64 * ceiling_lighting) as u8;
                    color.g = (color.g as f64 * ceiling_lighting) as u8;
                    color.b = (color.b as f64 * ceiling_lighting) as u8;

                    self.sdl_canvas.pixel(x as i16, y as i16, color).unwrap();
                    continue;
                }
            }
        }

    }

    fn raycast(&self, origin_x: f64, origin_y: f64, angle: f64) -> RaycastHit {
        // TODO
        // Fix the fuckin infinite ray bug

        let mut intersection_distance: f64 = 0.0;
        let mut x: f64 = 0.0;
        let mut y: f64 = 0.0;
        let mut tile_x: u32 = 0;
        let mut tile_y: u32 = 0;
        let mut tile_side: u8 = 0; // 0 for y, 1 for x

        let tile_size: f64 = 1.0;

        // Calculate the quadrant of the ray
        let angle: f64 = self.wrap_angle(angle);
        let is_ray_right: bool = angle > (TWO_PI * 0.75) || angle < (TWO_PI * 0.25);
        let is_ray_up: bool = angle < 0.0 || angle > std::f64::consts::PI;

        // Check for vertical (y axis) intersections

        let mut slope: f64 = angle.sin() / angle.cos();
        let mut delta_x: f64 = if is_ray_right { tile_size } else { -tile_size };
        let mut delta_y: f64 = delta_x * slope;

        // Calculate the ray starting position (first edge)
        let mut ray_position_x: f64 = if is_ray_right { origin_x.ceil() } else { origin_x.floor() };
        let mut ray_position_y: f64 = origin_y + (ray_position_x - origin_x) * slope;

        while (ray_position_x >= 0.0) && (ray_position_x < self.map.width as f64) && (ray_position_y >= 0.0) && (ray_position_y < self.map.height as f64) {
            let tile_map_x: u32 = f64::floor(ray_position_x + (if is_ray_right { 0.0 } else { -tile_size })) as u32;
            let tile_map_y: u32 = f64::floor(ray_position_y) as u32;

            if let Some(tile) = self.map.get_tile(tile_map_x, tile_map_y) {
                let mut distance_x: f64 = ray_position_x - origin_x;
                let mut distance_y: f64 = ray_position_y - origin_y;

                intersection_distance = distance_x.powi(2) + distance_y.powi(2);

                tile_side = 0;

                tile_x = tile.x;
                tile_y = tile.y;

                x = ray_position_x;
                y = ray_position_y;

                break;
            }

            ray_position_x += delta_x;
            ray_position_y += delta_y;
        }

        // Check for horizontal (x axis) intersections

        slope = angle.cos() / angle.sin();
        delta_y = if is_ray_up { -tile_size } else { tile_size }; // Vertical step amount
        delta_x = delta_y * slope; // Horizontal step amount

        // Calculate the ray starting position
        ray_position_y = if is_ray_up { f64::floor(origin_y) } else { f64::ceil(origin_y) };
        ray_position_x = origin_x + (ray_position_y - origin_y) * slope;

        while (ray_position_x >= 0.0) && (ray_position_x < self.map.width as f64) && (ray_position_y >= 0.0) && (ray_position_y < self.map.height as f64) {
            let tile_map_x: u32 = f64::floor(ray_position_x) as u32;
            let tile_map_y: u32 = f64::floor(ray_position_y + (if is_ray_up { -tile_size } else { 0.0 })) as u32;

            if let Some(tile) = self.map.get_tile(tile_map_x, tile_map_y) {
                let distance_x: f64 = ray_position_x - origin_x;
                let distance_y: f64 = ray_position_y - origin_y;
                let x_intersection_distance = distance_x.powi(2) + distance_y.powi(2);

                if (intersection_distance == 0.0) || (x_intersection_distance < intersection_distance) {
                    intersection_distance = x_intersection_distance;
                    tile_side = 1;

                    tile_x = tile.x;
                    tile_y = tile.y;

                    x = ray_position_x;
                    y = ray_position_y;
                }

                break;
            }

            ray_position_x += delta_x;
            ray_position_y += delta_y;
        }

        RaycastHit {
            x: x,
            y: y,
            tile_x: tile_x,
            tile_y: tile_y,
            tile_side: tile_side,
            distance: intersection_distance
        }
    }
}

fn main() {
    let mut game = Game::new();
    game.run();
}
