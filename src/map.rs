use std::{collections::HashMap, sync::Arc};

pub struct MapSize {
    cols: usize,
    rows: usize,
}

impl MapSize {
    pub fn cols(&self) -> usize {
        self.cols
    }
    pub fn rows(&self) -> usize {
        self.rows
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TileData {
    pub(crate) texture_path: &'static str,
}
impl TileData {
    pub fn new(texture_path: &'static str) -> Self {
        TileData { texture_path }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TileDataFC {
    pub(crate) texture_path_f: &'static str,
    pub(crate) texture_path_c: &'static str,
}
impl TileDataFC {
    pub fn new(texture_path_floor: &'static str, texture_path_ceiling: &'static str) -> Self {
        TileDataFC {
            texture_path_f: texture_path_floor,
            texture_path_c: texture_path_ceiling,
        }
    }
}

/// The type of a tile's 3D representation. Contains configuration data for the tile.
///
/// * wall_texture_path | ceiling_texture_path - the path to the image (from the 'res' directory) that will be applied to the 3D representation as a texture.
///
/// # Example
///
/// ```
/// let tile_type = TileType::Wall(TileData { texture_path: "wall.png" });
/// ```
#[derive(Clone, Copy, Debug)]
pub enum TileType {
    Wall(TileData),
    Floor(TileData),
    Ceiling(TileData),
    FloorCeiling(TileDataFC),
}

/// Holds a map's tile data, where the key is the number used to
pub type TileTypes = HashMap<u8, TileType>;

pub struct Map {
    tiles: Vec<Vec<u8>>,
    tile_types: TileTypes,
}

pub type Maps = HashMap<&'static str, Map>;

impl Map {
    pub fn new(tiles: Vec<Vec<u8>>, tile_types: TileTypes) -> Self {
        Self { tiles, tile_types }
    }
    pub fn size(&self) -> MapSize {
        MapSize {
            cols: self.tiles[0].len(),
            rows: self.tiles.len(),
        }
    }
    pub fn tiles(&self) -> &Vec<Vec<u8>> {
        &self.tiles
    }
    pub fn tile_id(&self, row: usize, col: usize) -> Option<u8> {
        Some(self.tiles[row][col])
    }
    pub fn tile_type(&self, tile_id: u8) -> Option<TileType> {
        // println!("{:?}", tile_id);
        self.tile_types.get(&tile_id).copied()
    }
    pub fn tile_types(&self) -> &TileTypes {
        &self.tile_types
    }
    pub fn img_path_count(&self) -> u16 {
        let mut count = 0;
        for (_, v) in &self.tile_types {
            match v {
                TileType::Wall(_) => count += 1,
                TileType::Ceiling(_) => count += 1,
                TileType::FloorCeiling(_) => count += 2,
                TileType::Floor(_) => count += 1,
            };
        }

        count
    }
}
