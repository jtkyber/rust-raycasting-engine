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

#[derive(Clone, Copy)]
pub enum TileType {
    Wall(&'static str),
    Floor(&'static str),
    Ceiling(&'static str),
    FloorCeiling(&'static str, &'static str),
}

pub type TileTypes = HashMap<u8, TileType>;

pub struct Map {
    layout: Vec<Vec<u8>>,
    tile_types: TileTypes,
}

impl Map {
    pub fn new(layout: Vec<Vec<u8>>, tile_types: TileTypes) -> Self {
        Self { layout, tile_types }
    }
    pub fn size(&self) -> MapSize {
        MapSize {
            cols: self.layout[0].len(),
            rows: self.layout.len(),
        }
    }
    pub fn layout(&self) -> &Vec<Vec<u8>> {
        &self.layout
    }
    pub fn tile_type(&self, row: usize, col: usize) -> Option<TileType> {
        let tile = &self.layout[row][col];
        self.tile_types.get(tile).copied()
    }
}
