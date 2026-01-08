#[cfg(test)]
mod tests;
use std::{f32::consts::PI, sync::Arc, vec};
mod math;
use anyhow::Ok;

use crate::{
    map::{Map, TileType},
    raycaster::math::{CustomMath, ray_tile_intersection},
};

enum AngleQuadrant {
    BottomRight,
    BottomLeft,
    TopLeft,
    TopRight,
}

#[derive(Clone, Copy)]
enum TileSide {
    Top,
    Left,
    Bottom,
    Right,
}

#[derive(Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
}

struct Ray {
    len: f32,
    angle: f32,
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

pub(crate) struct Raycaster {
    projection_plane_width: u32,
    projection_plane_height: u32,
    tile_size: u16,
    fov: u16,
    rays: Vec<Ray>,
    fish_table: Vec<f32>,
    player_position: Position,
    player_rotation: f32,
    maps: Arc<Vec<Map>>,
    current_map_index: usize,
}

impl Raycaster {
    pub fn new(
        config: &wgpu::SurfaceConfiguration,
        maps: Arc<Vec<Map>>,
    ) -> anyhow::Result<Raycaster> {
        let fov: u16 = 60;
        let ray_angles = get_ray_angles(fov, config.width)?;
        let fish_table = get_fish_table(config.width)?;

        Ok(Self {
            projection_plane_width: config.width,
            projection_plane_height: config.height,
            tile_size: 64,
            fov: fov,
            rays: ray_angles
                .iter()
                .map(|a| Ray {
                    len: f32::INFINITY,
                    angle: *a,
                    tile_index: None,
                    tile_intersection: None,
                    tile_type: None,
                    tile_side: None,
                })
                .collect(),
            fish_table,
            player_position: Position { x: 0.5, y: 0.5 },
            player_rotation: 0.0,
            maps: maps,
            current_map_index: 0,
        })
    }

    pub fn update(&mut self) -> () {
        let current_map = &self.maps[self.current_map_index];
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
    }
}

fn get_ray_angles(fov: u16, width: u32) -> anyhow::Result<Vec<f32>> {
    let fov = fov as f32;
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
