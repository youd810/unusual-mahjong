// /src/visuals/mod.rs

use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::core::{Tile, Honor, Wind};

#[derive(Resource, Default)]
pub struct TileModels {
    // maps the asset name (e.g., "pin1", "east") to the collected meshes and materials
    pub models: HashMap<String, Vec<(Handle<Mesh>, Handle<StandardMaterial>)>>,
}

// maps the tile enum variant to its corresponding string name in the gltf named nodes
pub fn get_tile_model_name(tile: &Tile) -> String {
    match tile {
        Tile::Man(number) => format!("man{}", number),
        Tile::Pin(number) => format!("pin{}", number),
        Tile::Sou(number) => format!("sou{}", number),
        Tile::Honor(honor) => match honor {
            Honor::East => "east".to_string(),
            Honor::South => "south".to_string(),
            Honor::West => "west".to_string(),
            Honor::North => "north".to_string(),
            Honor::White => "white_dragon".to_string(),
            Honor::Green => "green_dragon".to_string(),
            Honor::Red => "red_dragon".to_string(),
        },
    }
}

// maps seat wind to rotation offset index
pub fn get_seat_index(wind: &Wind) -> u8 {
    // east is seat 0 (bottom/south facing north), south is seat 1 (right facing west), etc.
    wind.to_num()
}

// returns rotation for a given seat index (0 = east/bottom, 1 = south/right, etc.)
pub fn get_seat_rotation(seat_index: u8) -> Quat {
    Quat::from_rotation_y(seat_index as f32 * std::f32::consts::FRAC_PI_2)
}

// returns seat translation offset from table center
pub fn get_seat_position(seat_index: u8, base_offset: Vec3) -> Vec3 {
    let rotation = get_seat_rotation(seat_index);
    rotation.mul_vec3(base_offset)
}
