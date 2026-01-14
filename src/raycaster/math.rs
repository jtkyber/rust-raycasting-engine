use std::ops::{Add, Rem, Sub};

use crate::raycaster::{Position, TileSide};

pub(crate) trait CustomMath {
    fn keep_in_range(&self, min: Self, max: Self) -> Self;
}

impl<T> CustomMath for T
where
    T: Copy + PartialOrd + Sub<Output = T> + Rem<Output = T> + Add<Output = T>,
{
    fn keep_in_range(&self, min: Self, max: Self) -> Self {
        let range_size = max - min;
        min + (((*self - min) % range_size + range_size) % range_size)
    }
}

pub(crate) fn ray_line_intersection(
    rx1: f32,
    ry1: f32,
    r: f32,
    ray_angle: f32,
    lx1: f32,
    ly1: f32,
    lx2: f32,
    ly2: f32,
) -> Option<Position> {
    let rx2 = rx1 + r * ray_angle.cos();
    let ry2 = ry1 + r * ray_angle.sin();

    let denom = (lx1 - lx2) * (ry1 - ry2) - (ly1 - ly2) * (rx1 - rx2);

    if denom == 0.0 {
        return None;
    }

    let t = ((lx1 - rx1) * (ry1 - ry2) - (ly1 - ry1) * (rx1 - rx2)) / denom;
    let u = ((lx1 - rx1) * (ly1 - ly2) - (ly1 - ry1) * (lx1 - lx2)) / denom;

    if (0.0..=1.0).contains(&t) && u >= 0.0 {
        let px = rx1 + u * (rx2 - rx1);
        let py = ry1 + u * (ry2 - ry1);
        return Some(Position { x: px, y: py });
    }

    None
}

pub(crate) struct IntersectionData {
    pub dist: f32,
    pub intersection: Position,
    pub side: TileSide,
}

pub(crate) fn ray_tile_intersection(
    rx1: f32,
    ry1: f32,
    row: usize,
    col: usize,
    tile_size: u16,
    ray_angle: f32,
    sides: [TileSide; 2],
) -> Option<IntersectionData> {
    let x1: f32 = (col as u16 * tile_size).into();
    let y1: f32 = (row as u16 * tile_size).into();

    let x2 = x1 + tile_size as f32;
    let y2 = y1;

    let x3 = x2;
    let y3 = y1 + tile_size as f32;

    let x4 = x1;
    let y4 = y3;

    let mut record = f32::INFINITY;
    let mut closest: Option<Position> = None;
    let mut side = TileSide::Top;

    let mut tx1;
    let mut ty1;
    let mut tx2;
    let mut ty2;

    for i in 0..2 {
        match sides[i] {
            TileSide::Top => {
                tx1 = x1;
                ty1 = y1;
                tx2 = x2;
                ty2 = y2;
            }
            TileSide::Right => {
                tx1 = x2;
                ty1 = y2;
                tx2 = x3;
                ty2 = y3;
            }
            TileSide::Bottom => {
                tx1 = x3;
                ty1 = y3;
                tx2 = x4;
                ty2 = y4;
            }
            TileSide::Left => {
                tx1 = x4;
                ty1 = y4;
                tx2 = x1;
                ty2 = y1;
            }
        }

        let intersection = ray_line_intersection(rx1, ry1, 1.0, ray_angle, tx1, ty1, tx2, ty2);

        if let Some(pos) = intersection {
            let dx = (rx1 - pos.x).abs();
            let dy = (ry1 - pos.y).abs();
            let d = (dx * dx + dy * dy).sqrt();

            record = d.min(record);
            if d <= record {
                record = d;
                closest = intersection;
                side = sides[i];
            }
        }
    }

    if let Some(pos) = closest {
        return Some(IntersectionData {
            dist: record,
            intersection: pos,
            side,
        });
    }

    None
}

#[cfg(test)]
mod math_tests {
    use std::f32::consts::PI;

    use super::*;

    #[test]
    fn keep_in_range() {
        let angle_in_radians: f32 = 8.56;
        let angle_in_range = angle_in_radians.keep_in_range(0.0, 2.0 * PI);
        let angle_rounded_to_hundreth = (angle_in_range * 100.0).round() / 100.0;
        assert_eq!(angle_rounded_to_hundreth, 2.28);
    }
}
