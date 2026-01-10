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

/// The type of a tile's 3D representation. Contains configuration data for the tile.
///
/// * wall_texture_path | ceiling_texture_path - the path to the image (from the 'res' directory) that will be applied to the 3D representation as a texture.
///
/// # Example
///
/// ```
/// let tile_type = TileType::Wall { wall_texture_path: "wall.png" };
/// ```
#[derive(Clone, Copy, Debug)]
pub enum TileType {
    Wall {
        wall_texture_path: &'static str,
    },
    Floor {
        floor_texture_path: &'static str,
    },
    Ceiling {
        ceiling_texture_path: &'static str,
    },
    FloorCeiling {
        floor_texture_path: &'static str,
        ceiling_texture_path: &'static str,
    },
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
    pub fn tile_type(&self, row: usize, col: usize) -> Option<TileType> {
        let tile = &self.tiles[row][col];
        self.tile_types.get(tile).copied()
    }
    pub fn tile_types(&self) -> &TileTypes {
        &self.tile_types
    }
}
