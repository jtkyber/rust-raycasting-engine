#[cfg(test)]
mod tests;
use std::{f32::consts::PI, sync::Arc, vec};
mod math;
use anyhow::Ok;
use glam::Vec2;
use winit::{
    event_loop::{self, ActiveEventLoop},
    keyboard::KeyCode,
};

use crate::{
    map::{Map, Maps, TileType},
    raycaster::math::{CustomMath, ray_tile_intersection},
    renderer::{self, Renderer},
};

const BYTES_PER_PIXEL: u8 = 4;

enum AngleQuadrant {
    BottomRight,
    BottomLeft,
    TopLeft,
    TopRight,
}

#[derive(Clone, Copy, Debug)]
enum TileSide {
    Top,    // 0
    Left,   // 1
    Bottom, // 2
    Right,  // 3
}

#[derive(Clone, Copy, Debug)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug)]
struct Ray {
    len: f32,
    angle: f32,
    fisheye_correction: f32,
    tile_index: Option<usize>,
    tile_intersection: Option<Position>,
    tile_id: Option<u8>,
    tile_type: Option<TileType>,
    tile_side: Option<TileSide>,
    tile_image_index: Option<usize>,
}

impl Ray {
    fn update_intersection(
        &mut self,
        len: f32,
        tile_index: Option<usize>,
        tile_intersection: Option<Position>,
        tile_type: Option<TileType>,
        tile_side: Option<TileSide>,
        tile_id: Option<u8>,
        tile_image_index: Option<usize>,
    ) {
        self.len = len;
        self.tile_index = tile_index;
        self.tile_intersection = tile_intersection;
        self.tile_type = tile_type;
        self.tile_side = tile_side;
        self.tile_id = tile_id;
        self.tile_image_index = tile_image_index;
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct WallInstance {
    screen_x: f32,
    top: f32,
    height: f32,
    tex_u: f32,
    tex_layer: u32,
    _pad: [u32; 3],
}

pub(crate) struct Raycaster {
    renderer: Renderer,
    projection_plane_width: u32,
    projection_plane_height: u32,
    projection_plane_y_center: u32,
    tile_size: u16,
    wall_height: u16,
    fov: f32,
    rays: Vec<Ray>,
    player_position: Position,
    player_rotation: f32,
    player_height: u16,
    player_dist_to_projection_plane: f32,
    maps: Arc<Maps>,
    current_map_key: &'static str,
}

impl Raycaster {
    pub fn new(
        renderer: Renderer,
        maps: Arc<Maps>,
        current_map_key: &'static str,
    ) -> anyhow::Result<Raycaster> {
        let config = renderer.config().clone();

        let fov: f32 = 60.0;
        let player_dist_to_projection_plane =
            config.width as f32 / 2.0 / (fov.to_radians() / 2.0).tan();
        let ray_angles = get_ray_angles(fov, config.width)?;
        let fish_table = get_fish_table(config.width)?;

        Ok(Self {
            renderer,
            projection_plane_width: config.width,
            projection_plane_height: config.height,
            projection_plane_y_center: config.height / 2,
            tile_size: 64,
            wall_height: 64,
            fov,
            rays: ray_angles
                .iter()
                .enumerate()
                .map(|(i, a)| Ray {
                    len: f32::INFINITY,
                    angle: *a,
                    fisheye_correction: fish_table[i],
                    tile_index: None,
                    tile_intersection: None,
                    tile_id: None,
                    tile_type: None,
                    tile_side: None,
                    tile_image_index: None,
                })
                .collect(),
            player_position: Position { x: 100.0, y: 100.0 },
            player_rotation: 10.0,
            player_height: 32,
            player_dist_to_projection_plane,
            maps: maps,
            current_map_key,
        })
    }

    pub fn update(&mut self) -> anyhow::Result<()> {
        self.update_rays()?;
        self.update_quads()?;

        self.renderer.render()?;

        Ok(())
    }

    fn update_rays(&mut self) -> anyhow::Result<()> {
        let current_map = &self.maps.get(self.current_map_key).unwrap();
        let map_size = current_map.size();
        let map_cols = map_size.cols();
        let map_rows = map_size.rows();

        for ray in &mut self.rays {
            let mut adjusted_angle = ray.angle + self.player_rotation.to_radians();
            adjusted_angle = adjusted_angle.keep_in_range(0.0, 2.0 * PI);

            let mut closest: Option<Position> = None;
            let mut record = f32::INFINITY;

            let ray_angle_quadrant = get_angle_quadrant(adjusted_angle);

            let sides_to_check: [TileSide; 2] = match ray_angle_quadrant {
                AngleQuadrant::BottomRight => [TileSide::Top, TileSide::Left],
                AngleQuadrant::BottomLeft => [TileSide::Top, TileSide::Right],
                AngleQuadrant::TopLeft => [TileSide::Right, TileSide::Bottom],
                AngleQuadrant::TopRight => [TileSide::Bottom, TileSide::Left],
            };

            let mut tile_index: Option<usize> = None;
            let mut tile_id: Option<u8> = None;
            let mut tile_type: Option<TileType> = None;
            let mut tile_side: Option<TileSide> = None;
            for row in 0..map_rows {
                for col in 0..map_cols {
                    tile_id = current_map.tile_id(row, col);
                    tile_type = current_map.tile_type(tile_id.unwrap());

                    match tile_type {
                        Some(TileType::Wall(_)) => (),
                        _ => continue,
                    }

                    let tile_intersection = ray_tile_intersection(
                        self.player_position.x,
                        self.player_position.y,
                        row,
                        col,
                        self.tile_size,
                        adjusted_angle,
                        sides_to_check,
                    );

                    if let Some(data) = tile_intersection {
                        if data.dist < record {
                            record = data.dist;
                            closest = Some(data.intersection);
                            tile_side = Some(data.side);
                            tile_index = Some(row * map_cols + col);
                        }
                    }
                }
            }

            if let (Some(intersection), Some(t_index), Some(t_id), Some(t_type), Some(t_side)) =
                (closest, tile_index, tile_id, tile_type, tile_side)
            {
                // println!("{:?}", tile_id);
                let texture_index = self
                    .renderer
                    .get_texture_index(t_id, &renderer::TextureCategory::Wall)?;

                ray.update_intersection(
                    record.floor(),
                    Some(t_index),
                    Some(intersection),
                    Some(t_type),
                    Some(t_side),
                    Some(t_id),
                    Some(texture_index),
                );
            } else {
                ray.update_intersection(record.floor(), None, None, None, None, None, None);
            }
        }

        Ok(())
    }

    fn update_quads(&mut self) -> anyhow::Result<()> {
        for (i, ray) in self.rays.iter().enumerate() {
            if let (Some(intersection), Some(tile_side), Some(tile_id)) =
                (ray.tile_intersection, ray.tile_side, ray.tile_id)
            {
                let dist = ray.len / ray.fisheye_correction;

                let ratio = self.player_dist_to_projection_plane / dist;
                let scale = (self.player_dist_to_projection_plane * self.wall_height as f32) / dist;
                let wall_bottom =
                    ratio * self.player_height as f32 + self.projection_plane_y_center as f32;
                let wall_top = wall_bottom - scale;
                let wall_height = wall_bottom - wall_top;

                // let adjusted_angle = ray.angle + self.player_rotation.to_radians();
                // let adjusted_angle = adjusted_angle.keep_in_range(0.0, 2.0 * PI);

                // let mut offset = match ray.tile_side {
                //
                // }

                let use_x_for_offset =
                    matches!(tile_side, TileSide::Top) || matches!(tile_side, TileSide::Bottom);

                // Tile-local offset for texture column start
                let offset = if use_x_for_offset {
                    let offset_temp =
                        (intersection.x.floor() as i32).rem_euclid(self.tile_size as i32);
                    // Mirror
                    (self.tile_size as i32) - offset_temp - 1
                } else {
                    (intersection.y.floor() as i32).rem_euclid(self.tile_size as i32)
                } as f32;

                let tex_u = (offset + 0.5) / (self.tile_size as f32);

                let tex_layer = self
                    .renderer
                    .get_texture_index(tile_id, &renderer::TextureCategory::Wall)?;

                let instance = WallInstance {
                    screen_x: i as f32,
                    top: wall_top as f32,
                    height: wall_height as f32,
                    tex_u,
                    tex_layer: tex_layer as u32,
                    _pad: [0u32; 3],
                };

                self.renderer.set_wall_instance(i, instance)?;
            };
        }

        Ok(())
    }

    pub fn renderer(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    pub fn handle_key(&mut self, _event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::ArrowRight, true) => {
                self.player_rotation += 0.01;
                println!("RIGHT");
            }
            (KeyCode::ArrowLeft, true) => {
                println!("LEFT");
                self.player_rotation -= 0.01;
            }
            _ => (),
        }
    }
}

fn get_ray_angles(fov: f32, width: u32) -> anyhow::Result<Vec<f32>> {
    let ray_inc: f32 = fov / width as f32;
    let mut angle: f32 = 0.0;
    let mut ray_angles: Vec<f32> = vec![];

    for _ in 0..width {
        let ray_angle: f32 = angle - fov / 2.0;
        ray_angles.push(ray_angle.to_radians());
        angle += ray_inc;
    }

    Ok(ray_angles)
}

fn get_fish_table(width: u32) -> anyhow::Result<Vec<f32>> {
    let width = width as f32;
    let half_neg: i32 = (-width / 2.0).floor() as i32;
    let half: i32 = (width / 2.0).floor() as i32;
    let mut fish_table: Vec<f32> = vec![0.0; width as usize];

    for n in half_neg..half {
        let radian: f32 = (n as f32 * PI) / (width * 3.0);
        fish_table[(n + half) as usize] = 1.0 / radian.cos();
    }

    Ok(fish_table)
}

fn get_angle_quadrant(angle: f32) -> AngleQuadrant {
    let ray_angle_quadrant_id: u8 = (angle / (PI / 2.0)).floor() as u8;
    match ray_angle_quadrant_id {
        0 => AngleQuadrant::BottomRight,
        1 => AngleQuadrant::BottomLeft,
        2 => AngleQuadrant::TopLeft,
        3 => AngleQuadrant::TopRight,
        _ => AngleQuadrant::BottomRight,
    }
}
