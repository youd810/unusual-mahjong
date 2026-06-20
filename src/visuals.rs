// ! discards aren't rendering on round 2 onwards

use bevy::prelude::*;
use bevy::gltf::{Gltf, GltfNode, GltfMesh};
use std::collections::{HashMap, HashSet};
use crate::components::*;
use crate::core::{Tile, Honor};
use crate::resources::Omniscience;

#[derive(Clone)]
pub struct TilePart {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    pub transform: Transform,
}

#[derive(Resource, Default)]
pub struct TileModels {
    // maps the asset name (e.g., "pin1") to its collected visual mesh parts
    pub models: HashMap<String, Vec<TilePart>>,
}

#[derive(Resource)]
pub struct GltfLoadState {
    pub handle: Handle<Gltf>,
}

#[derive(Component)]
pub struct VisualHandTile {
    pub owner: Entity,
}

#[derive(Component)]
pub struct VisualDiscardAttached;

pub struct VisualsPlugin;

impl Plugin for VisualsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileModels>()
            .add_systems(Startup, (setup_camera_and_light, start_loading_gltf))
            .add_systems(Update, (
                check_gltf_loaded,
                render_hands_system.run_if(resource_exists::<TileModels>),
                render_discards_system.run_if(resource_exists::<TileModels>),
            ));
    }
}

fn setup_camera_and_light(mut commands: Commands) {
    // spawn main camera looking at the center of the board
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 6.0, 7.5).looking_at(Vec3::new(0.0, -0.5, 0.0), Vec3::Y),
    ));

    // spawn directional light to simulate overhead room lighting
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 12000.0,
            ..default()
        },
        Transform::from_xyz(5.0, 15.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn start_loading_gltf(mut commands: Commands, asset_server: Res<AssetServer>) {
    let gltf_handle = asset_server.load("models/riichi_mahjong.glb");
    commands.insert_resource(GltfLoadState { handle: gltf_handle });
}

fn check_gltf_loaded(
    mut commands: Commands,
    load_state: Option<Res<GltfLoadState>>,
    gltf_assets: Res<Assets<Gltf>>,
    gltf_nodes: Res<Assets<GltfNode>>,
    gltf_meshes: Res<Assets<GltfMesh>>,
    mut tile_models: ResMut<TileModels>,
) {
    let Some(state) = load_state else { return };
    let Some(gltf) = gltf_assets.get(&state.handle) else { return };

    let tile_names = vec![
        "man1", "man2", "man3", "man4", "man5", "man6", "man7", "man8", "man9",
        "pin1", "pin2", "pin3", "pin4", "pin5", "pin6", "pin7", "pin8", "pin9",
        "sou1", "sou2", "sou3", "sou4", "sou5", "sou6", "sou7", "sou8", "sou9",
        "east", "south", "west", "north",
        "red_dragon", "green_dragon", "white_dragon",
    ];

    for name in &tile_names {
        let node_handle = gltf.named_nodes.get(*name).or_else(|| {
            gltf.named_nodes
                .iter()
                .find(|(key, _value)| key.starts_with(name))
                .map(|(_key, value)| value)
        });

        if let Some(found_handle) = node_handle {
            let mut parts = Vec::new();
            collect_tile_parts(
                found_handle,
                &gltf_nodes,
                &gltf_meshes,
                Transform::IDENTITY,
                &mut parts,
                true,
            );
            if !parts.is_empty() {
                tile_models.models.insert(name.to_string(), parts);
            }
        }
    }

    commands.remove_resource::<GltfLoadState>();
    println!("Visual models extracted successfully.");
}

fn collect_tile_parts(
    node_handle: &Handle<GltfNode>,
    gltf_nodes: &Assets<GltfNode>,
    gltf_meshes: &Assets<GltfMesh>,
    accumulated_transform: Transform,
    out_list: &mut Vec<TilePart>,
    is_root: bool,
) {
    let Some(node) = gltf_nodes.get(node_handle) else { return; };
    let local_transform = if is_root { Transform::IDENTITY } else { node.transform };
    let current_transform = accumulated_transform * local_transform;

    if let Some(mesh_handle) = &node.mesh && let Some(gltf_mesh) = gltf_meshes.get(mesh_handle) {
        for primitive in &gltf_mesh.primitives {
            if let Some(material_handle) = &primitive.material {
                out_list.push(TilePart {
                    mesh: primitive.mesh.clone(),
                    material: material_handle.clone(),
                    transform: current_transform,
                });
            }
        }
    }

    for child_handle in &node.children {
        collect_tile_parts(
            child_handle,
            gltf_nodes,
            gltf_meshes,
            current_transform,
            out_list,
            false,
        );
    }
}

fn get_tile_model_name(tile: &Tile) -> String {
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

// updates the player's hand tiles whenever hands, draws, or omniscience changes
fn render_hands_system(
    mut commands: Commands,
    players_query: Query<(Entity, &Hand, &Jikaze, Option<&DrawnTile>, Has<HumanPlayer>), Or<(Changed<Hand>, Changed<DrawnTile>)>>,
    omniscience: Res<Omniscience>,
    tile_models: Res<TileModels>,
    existing_visual_tiles: Query<(Entity, &VisualHandTile)>,
) {
    // track which players had their hands updated this frame
    let mut updated_players = HashSet::new();

    for (player_entity, hand, jikaze, maybe_drawn, is_human) in &players_query {
        updated_players.insert(player_entity);

        // despawn any existing visual hand tiles for this player
        for (visual_entity, visual_tile) in &existing_visual_tiles {
            if visual_tile.owner == player_entity {
                commands.entity(visual_entity).despawn();
            }
        }

        let seat_index = jikaze.0.to_num();
        let seat_rotation = Quat::from_rotation_y(seat_index as f32 * std::f32::consts::FRAC_PI_2);

        // calculate hand layout dimensions
        let spacing_x = 0.28;
        let tile_scale = 0.015;
        let total_tiles = hand.0.len() + if maybe_drawn.is_some() { 1 } else { 0 };
        let hand_width = (total_tiles as f32 - 1.0) * spacing_x;

        let local_start_x = -hand_width / 2.0;
        let base_hand_position = Vec3::new(0.0, 0.0, 3.2);

        // spawn tiles in hand
        for (index, tile) in hand.0.iter().enumerate() {
            let offset_x = local_start_x + (index as f32 * spacing_x);
            let local_position = Vec3::new(offset_x, 0.0, 0.0);
            let world_position = seat_rotation.mul_vec3(base_hand_position + local_position);

            let final_rotation = if is_human {
                // human tiles always face the camera (outward)
                seat_rotation
            } else if omniscience.0 {
                // bots are revealed: face them inward to look at the center/camera
                seat_rotation * Quat::from_rotation_y(std::f32::consts::PI)
            } else {
                // bots are hidden: face them outward so their backs face the center
                seat_rotation
            };

            spawn_tile_instance(
                &mut commands,
                &tile_models,
                tile,
                player_entity,
                world_position,
                final_rotation,
                tile_scale,
            );
        }

        // spawn the drawn tile separated slightly from the main hand
        if let Some(drawn) = maybe_drawn {
            let offset_x = (local_start_x + (hand.0.len() as f32 * spacing_x)) + 0.12;
            let local_position = Vec3::new(offset_x, 0.0, 0.0);
            let world_position = seat_rotation.mul_vec3(base_hand_position + local_position);

            let final_rotation = if is_human {
                seat_rotation
            } else if omniscience.0 {
                seat_rotation * Quat::from_rotation_y(std::f32::consts::PI)
            } else {
                seat_rotation
            };

            spawn_tile_instance(
                &mut commands,
                &tile_models,
                &drawn.0,
                player_entity,
                world_position,
                final_rotation,
                tile_scale,
            );
        }
    }
}


// helper to spawn a visual tile hierarchy using model parts
fn spawn_tile_instance(
    commands: &mut Commands,
    tile_models: &TileModels,
    tile: &Tile,
    owner: Entity,
    position: Vec3,
    rotation: Quat,
    scale: f32,
) {
    let name = get_tile_model_name(tile);
    let Some(parts) = tile_models.models.get(&name) else {
        println!("Warning: No visual model found for: {}", name);
        return;
    };

    let parent_entity = commands.spawn((
        VisualHandTile { owner },
        Transform {
            translation: position,
            rotation,
            scale: Vec3::splat(scale),
        },
        Visibility::default(),
        InheritedVisibility::default(),
    )).id();

    for part in parts {
        let child_entity = commands.spawn((
            Mesh3d(part.mesh.clone()),
            MeshMaterial3d(part.material.clone()),
            part.transform,
        )).id();
        commands.entity(parent_entity).add_child(child_entity);
    }
}

// updates the kawa discards layout on the table
fn render_discards_system(
    mut commands: Commands,
    discarded_tiles_query: Query<(Entity, &DiscardedTile, &DiscardedBy), Without<VisualDiscardAttached>>,
    players_query: Query<&Jikaze>,
    kawa_query: Query<&Kawa>,
    tile_models: Res<TileModels>,
) {
    for (discard_entity, discarded_tile, discarded_by) in &discarded_tiles_query {
        let Ok(jikaze) = players_query.get(discarded_by.0) else { continue };
        let Ok(kawa) = kawa_query.get(discarded_by.0) else { continue };

        // calculate index of this tile in the owner's kawa
        let Some(index) = kawa.0.iter().position(|tile| *tile == discarded_tile.0) else { continue };

        let seat_index = jikaze.0.to_num();
        let seat_rotation = Quat::from_rotation_y(seat_index as f32 * std::f32::consts::FRAC_PI_2);

        // kawa layout metrics (laying tiles flat in rows of 6)
        let spacing_x = 0.28;
        let spacing_z = 0.38;
        let tile_scale = 0.015;
        let row_index = index / 6;
        let col_index = index % 6;

        let base_kawa_position = Vec3::new(-0.7, 0.0, 1.2);
        let offset_x = col_index as f32 * spacing_x;
        // grow discard pile row-by-row towards the center of the table
        let offset_z = -(row_index as f32 * spacing_z);

        let local_position = base_kawa_position + Vec3::new(offset_x, 0.0, offset_z);
        let world_position = seat_rotation.mul_vec3(local_position);

        // rotate tile to lie flat face-up on the table
        let flat_rotation = seat_rotation * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);

        let name = get_tile_model_name(&discarded_tile.0);
        if let Some(parts) = tile_models.models.get(&name) {
            // override the position of the logical discard entity
            commands.entity(discard_entity).insert((
                Transform {
                    translation: world_position,
                    rotation: flat_rotation,
                    scale: Vec3::splat(tile_scale),
                },
                Visibility::default(),
                InheritedVisibility::default(),
                VisualDiscardAttached,
            ));

            // attach the mesh parts as children
            for part in parts {
                let child_entity = commands.spawn((
                    Mesh3d(part.mesh.clone()),
                    MeshMaterial3d(part.material.clone()),
                    part.transform,
                )).id();
                commands.entity(discard_entity).add_child(child_entity);
            }
        }
    }
}
