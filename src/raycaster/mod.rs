#[cfg(test)]
mod tests;
use std::{f32::consts::PI, sync::Arc, vec};
mod math;
use anyhow::Ok;

use crate::{
    map::{Map, Maps, TileType},
    raycaster::math::{CustomMath, ray_tile_intersection},
    renderer::Renderer,
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
    Top,
    Left,
    Bottom,
    Right,
}

#[derive(Clone, Copy, Debug)]
struct Position {
    x: f32,
    y: f32,
}

struct Ray {
    len: f32,
    angle: f32,
    fisheye_correction: f32,
    tile_index: Option<usize>,
    tile_intersection: Option<Position>,
    tile_type: Option<TileType>,
    tile_side: Option<TileSide>,
}

impl Ray {
    fn update_intersection(
        &mut self,
        len: f32,
        tile_index: Option<usize>,
        tile_intersection: Option<Position>,
        tile_type: Option<TileType>,
        tile_side: Option<TileSide>,
    ) {
        self.len = len;
        self.tile_index = tile_index;
        self.tile_intersection = tile_intersection;
        self.tile_type = tile_type;
        self.tile_side = tile_side;
    }
}

type Quad = [glam::Vec2; 4];

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
    wall_quads: Vec<Quad>,
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
                    tile_type: None,
                    tile_side: None,
                })
                .collect(),
            player_position: Position { x: 100.0, y: 100.0 },
            player_rotation: 10.0,
            player_height: 32,
            player_dist_to_projection_plane,
            maps: maps,
            current_map_key,
            wall_quads: Vec::new(),
        })
    }

    pub fn update(&mut self) -> anyhow::Result<()> {
        self.update_rays();

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
            let mut tile_type: Option<TileType> = None;
            let mut tile_side: Option<TileSide> = None;
            for row in 0..map_rows {
                for col in 0..map_cols {
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
                            tile_index = Some(row * map_cols + col);
                            tile_type = current_map.tile_type(row, col);
                            tile_side = Some(data.side);
                        }
                    }
                }
            }

            if let (Some(intersection), Some(t_index), Some(t_type), Some(t_side)) =
                (closest, tile_index, tile_type, tile_side)
            {
                println!("{:?}", t_side);
                ray.update_intersection(
                    record.floor(),
                    Some(t_index),
                    Some(intersection),
                    Some(t_type),
                    Some(t_side),
                );
            } else {
                ray.update_intersection(record.floor(), None, None, None, None);
            }
        }

        Ok(())
    }

    fn update_quads(&mut self) -> anyhow::Result<()> {
        for ray in &self.rays {
            // if let Some(_) = ray.tile_index {
            //     continue;
            // };

            let dist = ray.len / ray.fisheye_correction;

            let ratio = self.player_dist_to_projection_plane / dist;
            let scale = (self.player_dist_to_projection_plane * self.wall_height as f32) / dist;
            let wall_bottom =
                ratio * self.player_height as f32 + self.projection_plane_y_center as f32;
            let wall_top = wall_bottom - scale;
            let wall_height = wall_bottom - wall_top;

            let adjusted_angle = ray.angle + self.player_rotation.to_radians();
            let adjusted_angle = adjusted_angle.keep_in_range(0.0, 2.0 * PI);
        }

        Ok(())
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
