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
pub struct VisualKawaTile {
    pub owner: Entity,
}

#[derive(Component)]
pub struct VisualMentsuTile {
    pub owner: Entity,
}

#[derive(Component)]
pub struct VisualWallTile;

pub struct VisualsPlugin;

impl Plugin for VisualsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileModels>()
            .add_systems(Startup, (setup_camera_and_light, start_loading_gltf))
            .add_systems(Update, (
                check_gltf_loaded,
                render_hands_system.run_if(resource_exists::<TileModels>),
                render_kawa_system.run_if(resource_exists::<TileModels>),
                render_wall_system.run_if(resource_exists::<TileModels>),
                render_open_mentsu_system.run_if(resource_exists::<TileModels>),
                cleanup_orphaned_visuals_system,
            ));
    }
}

fn setup_camera_and_light(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 6.0, 7.5).looking_at(Vec3::new(0.0, -0.5, 0.0), Vec3::Y),
    ));
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

    // strictly strip ONLY the root node to remove the artist's table placements
    // DO NOT override child rotations, ensuring front and back meshes stay connected
    let local_transform = if is_root { Transform::IDENTITY } else { node.transform };
    let current_transform = accumulated_transform * local_transform;

    if let Some(mesh_handle) = &node.mesh {
        if let Some(gltf_mesh) = gltf_meshes.get(mesh_handle) {
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
    }

    for child_handle in &node.children {
        collect_tile_parts(child_handle, gltf_nodes, gltf_meshes, current_transform, out_list, false);
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

fn render_hands_system(
    mut commands: Commands,
    players_query: Query<(Entity, &Hand, &Seat, Option<&DrawnTile>, Has<HumanPlayer>, Ref<Hand>, Option<Ref<DrawnTile>>)>,
    omniscience: Res<Omniscience>,
    tile_models: Res<TileModels>,
    existing_visual_tiles: Query<(Entity, &VisualHandTile)>,
) {
    if tile_models.models.is_empty() { return; }
    let force_redraw = tile_models.is_changed();

    for (player_entity, hand, seat, maybe_drawn, is_human, ref_hand, maybe_ref_drawn) in &players_query {
        let hand_changed = ref_hand.is_changed();
        let drawn_changed = maybe_ref_drawn.map(|reference| reference.is_changed()).unwrap_or(false);

        if force_redraw || hand_changed || drawn_changed {
            for (visual_entity, visual_tile) in &existing_visual_tiles {
                if visual_tile.owner == player_entity {
                    commands.entity(visual_entity).despawn();
                }
            }

            let seat_index = seat.0;
            let seat_rotation = Quat::from_rotation_y(seat_index as f32 * std::f32::consts::FRAC_PI_2);

            let spacing_x = 0.18;
            let tile_scale = 0.015;
            let total_tiles = hand.0.len() + if maybe_drawn.is_some() { 1 } else { 0 };
            let hand_width = (total_tiles as f32 - 1.0) * spacing_x;

            let local_start_x = -hand_width / 2.0;
            let base_hand_position = Vec3::new(0.0, 0.0, 3.2);

            // base mesh is already standing up
            for (index, tile) in hand.0.iter().enumerate() {
                let offset_x = local_start_x + (index as f32 * spacing_x);
                let local_position = Vec3::new(offset_x, 0.0, 0.0);
                let world_position = seat_rotation.mul_vec3(base_hand_position + local_position);

                let final_rotation = if !is_human && omniscience.0 {
                    // tiles facing inwards
                    seat_rotation * Quat::from_rotation_y(std::f32::consts::PI)
                } else {
                    // tiles facing outwards
                    seat_rotation
                };

                spawn_tile_instance(&mut commands, &tile_models, tile, player_entity, world_position, final_rotation, tile_scale, true);
            }

            if let Some(drawn) = maybe_drawn {
                let offset_x = (local_start_x + (hand.0.len() as f32 * spacing_x)) + 0.12;
                let local_position = Vec3::new(offset_x, 0.0, 0.0);
                let world_position = seat_rotation.mul_vec3(base_hand_position + local_position);

                let final_rotation = if is_human || omniscience.0 {
                    seat_rotation
                } else {
                    seat_rotation * Quat::from_rotation_y(std::f32::consts::PI)
                };

                spawn_tile_instance(&mut commands, &tile_models, &drawn.0, player_entity, world_position, final_rotation, tile_scale, true);
            }
        }
    }
}


fn render_kawa_system(
    mut commands: Commands,
    players_query: Query<(Entity, &Kawa, &Seat, Ref<Kawa>)>,
    tile_models: Res<TileModels>,
    existing_kawa_tiles: Query<(Entity, &VisualKawaTile)>,
) {
    if tile_models.models.is_empty() { return; }
    let force_redraw = tile_models.is_changed();

    for (player_entity, kawa, seat, ref_kawa) in &players_query {
        if force_redraw || ref_kawa.is_changed() {
            for (visual_entity, visual_tile) in &existing_kawa_tiles {
                if visual_tile.owner == player_entity {
                    commands.entity(visual_entity).despawn();
                }
            }

            let seat_index = seat.0;
            let seat_rotation = Quat::from_rotation_y(seat_index as f32 * std::f32::consts::FRAC_PI_2);

            let spacing_x = 0.19;
            let spacing_z = 0.24;
            let tile_scale = 0.015;

            // facing up, head facing center
            let flat_rotation = seat_rotation * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);


            for (index, tile) in kawa.0.iter().enumerate() {
                let row_index = index / 6;
                let col_index = index % 6;

                // kinda center-ish
                let base_kawa_position = Vec3::new(-0.5, 0.0, 1.0);

                let offset_x = col_index as f32 * spacing_x;
                // new row descending
                let offset_z = row_index as f32 * spacing_z;

                let local_position = base_kawa_position + Vec3::new(offset_x, 0.0, offset_z);
                let world_position = seat_rotation.mul_vec3(local_position);

                spawn_tile_instance(&mut commands, &tile_models, tile, player_entity, world_position, flat_rotation, tile_scale, false);
            }
        }
    }
}

// TODO: haipai https://www.youtube.com/watch?v=7BNe02MWLg0
pub fn render_wall_system(
    mut commands: Commands,
    wall: Option<Res<crate::resources::Wall>>,
    tile_models: Res<TileModels>,
    existing_wall_tiles: Query<Entity, With<VisualWallTile>>,
) {
    let Some(wall) = wall else { return };
    if tile_models.models.is_empty() { return; }

    // Run if the wall changed OR if the glTF models just finished loading
    if !wall.is_changed() && !tile_models.is_changed() { return; }

    // clear the old wall meshes
    for entity in &existing_wall_tiles {
        commands.entity(entity).despawn();
    }

    let tile_scale = 0.015;
    let stack_spacing = 0.19; 
    let tile_height = 0.14;
    let wall_radius = 1.9;

    let base_idx = wall.tiles.len() - 14 + wall.rinshan_max;

    for index in 0..wall.tiles.len() {
        // skip standard draws
        if index < wall.head { continue; }

        // skip rinshan draws
        let is_rinshan_zone = index >= (wall.tiles.len() - 14) && index < base_idx;
        if is_rinshan_zone {
            let rinshan_offset = index - (wall.tiles.len() - 14);
            if rinshan_offset < wall.rinshan_draws { continue; }
        }

         let dead_wall_start = wall.tiles.len() - 14;

        // remap linear index to a global stack position (0–67)
        let (global_stack, is_bottom) = if index < dead_wall_start {
            // live wall: index 0 placed next to dead wall, growing away from it
            let last_live_stack = (dead_wall_start / 2) - 1; // stack 60
            (last_live_stack - (index / 2), index % 2 != 0)
        } else {
            // dead wall: continues from where live wall ends
            let dw_offset = index - dead_wall_start;
            ((dead_wall_start / 2) + (dw_offset / 2), dw_offset % 2 != 0)
        };

        let side = global_stack / 17;
        let stack = global_stack % 17;

        let side_rotation = Quat::from_rotation_y(side as f32 * std::f32::consts::FRAC_PI_2);

        // center the 17 stacks. 
        //wall_radius is positive so Side 0 is at the bottom (east)
        let dead_wall_start = wall.tiles.len() - 14;
        let is_dead_wall = index >= dead_wall_start;

        let local_x = (stack as f32 - 8.0) * stack_spacing + if is_dead_wall { stack_spacing * 0.6 } else { 0.0 };
        let local_y = if is_bottom { 0.0 } else { tile_height };
        let local_z = wall_radius;

        

        let local_position = Vec3::new(local_x, local_y, local_z);
        let world_position = side_rotation.mul_vec3(local_position);

        let is_dora = index >= base_idx
            && index < base_idx + (wall.dora_count * 2)
            && (index - base_idx) % 2 == 0;

        // 1. placement rotation: All tiles face down to keep the glTF pivots perfectly uniform
        let placement_rotation = side_rotation * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);

        // 2. visual rotation: Only applied to the mesh itself
        let visual_rotation = if is_dora {
            // flips the mesh 180 degrees over its local axis so it faces up, without moving the pivot
            placement_rotation * Quat::from_rotation_x(std::f32::consts::PI)
        } else {
            placement_rotation
        };

        let tile = &wall.tiles[index];
        let name = get_tile_model_name(tile);

        if let Some(parts) = tile_models.models.get(&name) {
            for part in parts {
                // apply placement_rotation to the offset so the wall geometry remains a perfect square
                let part_offset = placement_rotation.mul_vec3(part.transform.translation * tile_scale);

                let part_transform = Transform {
                    translation: world_position + part_offset,
                    rotation: visual_rotation * part.transform.rotation,
                    scale: Vec3::splat(tile_scale) * part.transform.scale,
                };

                commands.spawn((
                    VisualWallTile,
                    Mesh3d(part.mesh.clone()),
                    MeshMaterial3d(part.material.clone()),
                    part_transform,
                    Visibility::default(),
                    InheritedVisibility::default(),
                ));
            }
        }
    }
}


// TODO: called tile indicator
fn render_open_mentsu_system(
    mut commands: Commands,
    players_query: Query<(Entity, &OpenMentsu, &Seat, Ref<OpenMentsu>)>,
    tile_models: Res<TileModels>,
    existing_mentsu_tiles: Query<(Entity, &VisualMentsuTile)>,
) {
    if tile_models.models.is_empty() { return; }
    let force_redraw = tile_models.is_changed();

    for (player_entity, open_mentsu, seat, ref_mentsu) in &players_query {
        if force_redraw || ref_mentsu.is_changed() {
            for (visual_entity, visual_tile) in &existing_mentsu_tiles {
                if visual_tile.owner == player_entity {
                    commands.entity(visual_entity).despawn();
                }
            }

            let seat_index = seat.0;
            let seat_rotation = Quat::from_rotation_y(seat_index as f32 * std::f32::consts::FRAC_PI_2);

            let spacing_x = 0.19;
            let tile_scale = 0.015;

            let mut current_offset_x = 2.0;
            let base_position = Vec3::new(0.0, 0.0, 3.2);

            let flat_rotation = seat_rotation * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);

            for mentsu in open_mentsu.0.iter() {
                for tile in mentsu.tiles() {
                    let local_position = Vec3::new(current_offset_x, 0.0, 0.0);
                    let world_position = seat_rotation.mul_vec3(base_position + local_position);

                    spawn_mentsu_instance(&mut commands, &tile_models, tile, player_entity, world_position, flat_rotation, tile_scale);

                    current_offset_x += spacing_x;
                }
                current_offset_x += 0.1;
            }
        }
    }
}

fn spawn_tile_instance(
    commands: &mut Commands,
    tile_models: &TileModels,
    tile: &Tile,
    owner: Entity,
    position: Vec3,
    rotation: Quat,
    scale: f32,
    is_hand: bool,
) {
    let name = get_tile_model_name(tile);
    let Some(parts) = tile_models.models.get(&name) else { return; };

    for part in parts {
        let part_transform = Transform {
            translation: position + rotation.mul_vec3(part.transform.translation * scale),
            rotation: rotation * part.transform.rotation,
            scale: Vec3::splat(scale) * part.transform.scale,
        };

        if is_hand {
            commands.spawn((
                VisualHandTile { owner },
                Mesh3d(part.mesh.clone()),
                MeshMaterial3d(part.material.clone()),
                part_transform,
                Visibility::default(),
                InheritedVisibility::default(),
            ));
        } else {
            commands.spawn((
                VisualKawaTile { owner },
                Mesh3d(part.mesh.clone()),
                MeshMaterial3d(part.material.clone()),
                part_transform,
                Visibility::default(),
                InheritedVisibility::default(),
            ));
        }
    }
}

fn spawn_mentsu_instance(
    commands: &mut Commands,
    tile_models: &TileModels,
    tile: &Tile,
    owner: Entity,
    position: Vec3,
    rotation: Quat,
    scale: f32,
) {
    let name = get_tile_model_name(tile);
    let Some(parts) = tile_models.models.get(&name) else { return; };

    for part in parts {
        let part_transform = Transform {
            translation: position + rotation.mul_vec3(part.transform.translation * scale),
            rotation: rotation * part.transform.rotation,
            scale: Vec3::splat(scale) * part.transform.scale,
        };

        commands.spawn((
            VisualMentsuTile { owner },
            Mesh3d(part.mesh.clone()),
            MeshMaterial3d(part.material.clone()),
            part_transform,
            Visibility::default(),
            InheritedVisibility::default(),
        ));
    }
}

fn cleanup_orphaned_visuals_system(
    mut commands: Commands,
    visual_hands: Query<(Entity, &VisualHandTile)>,
    visual_kawa: Query<(Entity, &VisualKawaTile)>,
    visual_mentsu: Query<(Entity, &VisualMentsuTile)>,
    players_query: Query<&Hand>,
) {
    for (visual_entity, visual_tile) in &visual_hands {
        if players_query.get(visual_tile.owner).is_err() {
            commands.entity(visual_entity).despawn();
        }
    }
    for (visual_entity, visual_tile) in &visual_kawa {
        if players_query.get(visual_tile.owner).is_err() {
            commands.entity(visual_entity).despawn();
        }
    }
    for (visual_entity, visual_tile) in &visual_mentsu {
        if players_query.get(visual_tile.owner).is_err() {
            commands.entity(visual_entity).despawn();
        }
    }
}
