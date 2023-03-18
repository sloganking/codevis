use std::{collections::HashMap, path::PathBuf};

use image::{Rgba, RgbaImage};

/// Turns pixel coordinates into tile coordinates
/// an x of 0 should return an x of 0.
/// and x of 255 should return an x of 255.
/// an x of 256 should return an x of 0.
/// an x of 257 should return 1.
/// an x of -1 should return 255.
/// an x of -2 should return 254.
/// an x of -256 should return 0.
/// an x of -257 should return 255.
pub fn to_tile_location(x: i32, y: i32) -> (u32, u32) {
    let x = if x >= 0 {
        x % 256
    } else {
        let mut result = x % 256;
        if result < 0 {
            result += 256;
        }
        result
    };
    let y = if y >= 0 {
        y % 256
    } else {
        let mut result = y % 256;
        if result < 0 {
            result += 256;
        }
        result
    };

    (x.try_into().unwrap(), y.try_into().unwrap())
}

pub struct TileCache {
    tile_path: PathBuf,
    cached_tiles: HashMap<(i32, i32), (usize, RgbaImage)>,
    cached_tile_limit: usize,
    cached_tile_nonce: usize,
}

impl TileCache {
    pub fn new(tile_path: PathBuf, cached_tile_limit: usize) -> Self {
        Self {
            tile_path,
            cached_tiles: HashMap::new(),
            cached_tile_limit,
            cached_tile_nonce: 0,
        }
    }

    fn add_tile_to_cache(&mut self, tile: RgbaImage, x: i32, y: i32) {
        println!("tile cache size: {}", self.cached_tiles.len());
        if let std::collections::hash_map::Entry::Vacant(e) = self.cached_tiles.entry((x, y)) {
            e.insert((self.cached_tile_nonce, tile));
            self.cached_tile_nonce += 1;
            self.trim_cache();
        }
    }

    /// gets a tile from the cache or loads it from disk
    /// creates a new tile if it doesn't exist
    fn get_tile(&mut self, x: i32, y: i32) -> &mut RgbaImage {
        if !self.cached_tiles.contains_key(&(x, y)) {
            let tile_path = self.tile_path.join(format!("{x},{y}.png"));
            let tile = if tile_path.exists() {
                image::open(tile_path).unwrap().to_rgba8()
            } else {
                RgbaImage::new(256, 256)
            };

            self.add_tile_to_cache(tile, x, y);
        }

        &mut self.cached_tiles.get_mut(&(x, y)).unwrap().1
    }

    /// saves all tiles in the cache to disk
    pub fn save_all(&self) {
        for ((x, y), (_tile_nonce, tile)) in &self.cached_tiles {
            let this_tile_path = self.tile_path.join(format!("{x},{y}.png"));
            tile.save(this_tile_path).unwrap();
        }
    }

    pub fn put_pixel(&mut self, x: i32, y: i32, pixel: Rgba<u8>) {
        let mut tile_x = x / 256;
        let mut tile_y = y / 256;

        // offset for negatives.
        if x % 256 != 0 && x < 0 {
            tile_x -= 1;
        }
        if y % 256 != 0 && y < 0 {
            tile_y -= 1;
        }

        let tile = self.get_tile(tile_x, tile_y);

        let (tile_pixel_x, tile_pixel_y) = to_tile_location(x, y);

        tile.put_pixel(tile_pixel_x, tile_pixel_y, pixel);
    }

    fn get_pixel(&mut self, _x: i32, _y: i32) -> Rgba<u8> {
        // let tile = self.get_tile(x / 256, y / 256);
        // tile.get_pixel((x % 256) as u32, (y % 256) as u32).clone()
        todo!();
    }

    // removes tiles from the cache if there are more than the limit.
    fn trim_cache(&mut self) {
        if self.cached_tiles.len() > self.cached_tile_limit {
            self.cached_tiles.retain(|(x, y), (this_tile_nonce, tile)| {
                let should_retain =
                    self.cached_tile_limit >= self.cached_tile_nonce - *this_tile_nonce;

                if !should_retain {
                    // save tile to disk
                    let this_tile_path = self.tile_path.join(format!("{x},{y}.png"));
                    tile.save(this_tile_path).unwrap();

                    // remove tile from cache
                    return false;
                }
                should_retain
            });
        }
    }
}
