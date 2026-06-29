use bevy::prelude::*;
use bevy::gltf::{Gltf, GltfNode, GltfMesh};
use std::collections::{HashMap, HashSet};
use crate::components::*;
use crate::core::*;
use crate::resources::*;

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


#[derive(Component)]
pub struct VisualRiichiStick {
    pub owner: Entity,
}

#[derive(Component)]
pub struct VisualNukidoraTile {
    pub owner: Entity,
}

pub struct VisualsPlugin;

impl Plugin for VisualsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileModels>()
            .add_systems(Startup, (setup_camera_and_light, start_loading_gltf))
            .add_systems(PostUpdate, (
                check_gltf_loaded,
                process_animations_system,
                (
                    render_kawa_system,
                    render_hands_system,
                    sync_animation_busy_system, // locked in at the end of the chain
                ).chain().run_if(resource_exists::<TileModels>),

                render_wall_system.run_if(resource_exists::<TileModels>),
                render_open_mentsu_system.run_if(resource_exists::<TileModels>),
                render_nukidora_system.run_if(resource_exists::<TileModels>),
                cleanup_orphaned_visuals_system,
                spawn_riichi_stick_system,
                cleanup_riichi_sticks_system,
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

pub fn render_hands_system(
    mut commands: Commands,
    players_query: Query<(Entity, &Hand, &Seat, Option<&DrawnTile>, Has<HumanPlayer>, Ref<Hand>, Option<Ref<DrawnTile>>)>,
    omniscience: Res<Omniscience>,
    tile_models: Res<TileModels>,
    mut existing_slots: Query<(Entity, &mut TileSlot, &Transform)>,
    mut busy: ResMut<AnimationBusy>,
) {
    if tile_models.models.is_empty() { return; }
    let force_redraw = tile_models.is_changed();

    for (player_entity, hand, seat, maybe_drawn, is_human, ref_hand, maybe_ref_drawn) in &players_query {
        let hand_changed = ref_hand.is_changed();
        let drawn_changed = maybe_ref_drawn.map(|reference| reference.is_changed()).unwrap_or(false);

        if force_redraw || hand_changed || drawn_changed {
            let seat_index = seat.0;
            let seat_rotation = Quat::from_rotation_y(seat_index as f32 * std::f32::consts::FRAC_PI_2);

            let spacing_x = 0.18;
            let tile_scale = 0.015;
            let total_tiles = hand.0.len() + if maybe_drawn.is_some() { 1 } else { 0 };
            let hand_width = (total_tiles as f32 - 1.0) * spacing_x;
            let local_start_x = -hand_width / 2.0;
            let base_hand_position = Vec3::new(0.0, 0.0, 3.2);

            // Combine logical hand and drawn tile into one list for target calculation
            let mut logical_target_slots = Vec::new();
            for (index, tile) in hand.0.iter().enumerate() {
                logical_target_slots.push((index, tile, false)); // false = not drawn
            }
            if let Some(drawn) = maybe_drawn {
                logical_target_slots.push((hand.0.len(), &drawn.0, true)); // true = drawn
            }

            // Track which existing entities we've matched so we don't reuse them
            let mut claimed_entities = HashSet::new();

            for (i, tile, is_drawn_tile) in logical_target_slots {
                let offset_x = if is_drawn_tile {
                    local_start_x + (hand.0.len() as f32 * spacing_x) + 0.12
                } else {
                    local_start_x + (i as f32 * spacing_x)
                };

                let local_position = Vec3::new(offset_x, 0.0, 0.0);
                let target_position = seat_rotation.mul_vec3(base_hand_position + local_position);

                let target_rotation = if !is_human && omniscience.0 {
                    seat_rotation * Quat::from_rotation_y(std::f32::consts::PI)
                } else {
                    seat_rotation
                };

                // Find a matching existing visual tile
                let mut matched_entity = None;
                for (vis_entity, vis_slot, _vis_transform) in existing_slots.iter_mut() {
                    if vis_slot.owner == player_entity
                        && vis_slot.zone == TileZone::Hand
                        && vis_slot.tile == *tile
                        && !claimed_entities.contains(&vis_entity)
                    {
                        matched_entity = Some(vis_entity);
                        break;
                    }
                }

                let target_transform = Transform::from_translation(target_position).with_rotation(target_rotation);

                if let Some(vis_ent) = matched_entity {
                    // Update existing tile and animate it
                    claimed_entities.insert(vis_ent);

                    // Update its slot index
                    if let Ok((_, mut slot, _)) = existing_slots.get_mut(vis_ent) {
                        slot.index = i;
                    }

                    commands.entity(vis_ent).insert(AnimateTo {
                        target_transform,
                        speed: 15.0, // Adjust for slide speed
                    });
                    busy.0 += 1;
                } else {
                    // It's a new tile! (Spawn it slightly above and animate it dropping into place)
                    let spawn_pos = target_position + Vec3::new(0.0, 1.0, 0.0);
                    if let Some(new_ent) = spawn_tile_instance(
                        &mut commands, &tile_models, tile, player_entity,
                        TileZone::Hand, i, spawn_pos, target_rotation, tile_scale
                    ) {
                        commands.entity(new_ent).insert(AnimateTo {
                            target_transform,
                            speed: 10.0,
                        });
                        busy.0 += 1;
                    }
                }
            }

            // Clean up any tiles that belonged to this player's hand but weren't claimed
            // (e.g. the tile you just discarded)
            for (vis_entity, vis_slot, _) in existing_slots.iter() {
                if vis_slot.owner == player_entity
                    && vis_slot.zone == TileZone::Hand
                    && !claimed_entities.contains(&vis_entity)
                {
                    commands.entity(vis_entity).despawn();
                }
            }
        }
    }
}


// TODO: despawn called tiles from kawa
// ! either that or stick with rendering the vector and ditch the entity logic
pub fn render_kawa_system(
    mut commands: Commands,
    players_query: Query<(Entity, &Kawa, &Seat, Ref<Kawa>, Option<&Riichi>, Option<Ref<CalledKawaIndices>>)>,
    current_discard_query: Query<(&DiscardedBy, &IsTsumogiri), With<CurrentDiscard>>,
    tile_models: Res<TileModels>,
    mut existing_slots: Query<(Entity, &mut TileSlot, &Transform)>,
    mut busy: ResMut<AnimationBusy>,
) {
    if tile_models.models.is_empty() { return; }
    let force_redraw = tile_models.is_changed();

    for (player_entity, kawa, seat, ref_kawa, maybe_riichi, maybe_called) in &players_query {

        // Check if either the Kawa array OR the called indices updated
        let kawa_changed = ref_kawa.is_changed();
        let called_changed = maybe_called.as_ref().is_some_and(|c| c.is_changed() || c.is_added());

        if force_redraw || kawa_changed || called_changed {
            let seat_index = seat.0;
            let seat_rotation = Quat::from_rotation_y(seat_index as f32 * std::f32::consts::FRAC_PI_2);

            let spacing_x = 0.19;
            let spacing_z = 0.24;
            let tile_scale = 0.015;
            let flat_rotation = seat_rotation * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);

            let riichi_index = maybe_riichi.map(|r| kawa.0.len().saturating_sub(1 + r.turns_since as usize));

            // Extract the actual vector out of the Ref wrapper
            let skipped_indices = maybe_called.as_ref().map(|c| c.0.clone()).unwrap_or_default();

            // calculate the visual position of the riichi tile to shift the row correctly
            let mut r_visual_idx = 0;
            if let Some(r_idx) = riichi_index {
                if !skipped_indices.contains(&r_idx) {
                    for i in 0..r_idx {
                        if !skipped_indices.contains(&i) { r_visual_idx += 1; }
                    }
                }
            }

            let mut visual_index = 0;
            let mut claimed_entities = std::collections::HashSet::new();

            for (index, tile) in kawa.0.iter().enumerate() {
                if skipped_indices.contains(&index) { continue; } // skip called tile

                let row_index = visual_index / 6;
                let col_index = visual_index % 6;

                // kinda center-ish
                let base_kawa_position = Vec3::new(-0.5, 0.0, 1.0);

                let mut offset_x = col_index as f32 * spacing_x;

                if let Some(r_idx) = riichi_index {
                    if !skipped_indices.contains(&r_idx) {
                        let r_row = r_visual_idx / 6;
                        let r_col = r_visual_idx % 6;

                        if row_index == r_row {
                            if col_index == r_col {
                                offset_x += spacing_x * 0.15;
                            } else if col_index > r_col {
                                offset_x += spacing_x * 0.3;
                            }
                        }
                    }
                }

                // new row descending
                let offset_z = row_index as f32 * spacing_z;

                let local_position = base_kawa_position + Vec3::new(offset_x, 0.0, offset_z);
                let target_position = seat_rotation.mul_vec3(local_position);

                let is_riichi_tile = Some(index) == riichi_index;
                let target_rotation = if is_riichi_tile {
                    flat_rotation * Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)
                } else {
                    flat_rotation
                };

                let target_transform = Transform::from_translation(target_position).with_rotation(target_rotation);

                // 1. Try to find a tile ALREADY in the Kawa
                let mut matched_entity = None;
                for (vis_entity, vis_slot, _) in existing_slots.iter() {
                    if vis_slot.owner == player_entity
                        && vis_slot.zone == TileZone::Kawa
                        && vis_slot.index == index
                    {
                        matched_entity = Some(vis_entity);
                        break;
                    }
                }

                // 2. If not in Kawa, look for an orphaned tile in the HAND (this creates the flying animation!)
                if matched_entity.is_none() {
                    // Check if this specific tile is the one we JUST discarded this frame
                    let mut is_tsumogiri = false;
                    if index == kawa.0.len() - 1 {
                        for (discarded_by, tsumogiri_flag) in &current_discard_query {
                            if discarded_by.0 == player_entity {
                                is_tsumogiri = tsumogiri_flag.0;
                            }
                        }
                    }

                    let mut best_match = None;
                    let mut best_score = -999;

                    for (vis_entity, vis_slot, _) in existing_slots.iter() {
                        if vis_slot.owner == player_entity
                            && vis_slot.zone == TileZone::Hand
                            && vis_slot.tile == *tile
                            && !claimed_entities.contains(&vis_entity)
                        {
                            // tegawari = lowest index; tsumogiri = highest index
                            let score = if is_tsumogiri {
                                vis_slot.index as i32
                            } else {
                                -(vis_slot.index as i32)
                            };

                            if best_match.is_none() || score > best_score {
                                best_score = score;
                                best_match = Some(vis_entity);
                            }
                        }
                    }
                    matched_entity = best_match;
                }

                if let Some(vis_ent) = matched_entity {
                    claimed_entities.insert(vis_ent);
                    if let Ok((_, mut slot, _)) = existing_slots.get_mut(vis_ent) {
                        slot.zone = TileZone::Kawa; // Steal it from the hand!
                        slot.index = index;
                    }
                    commands.entity(vis_ent).insert(AnimateTo {
                        target_transform,
                        speed: 12.0,
                    });
                    busy.0 += 1;
                } else {
                    // Fallback spawn if we couldn't steal from the hand
                    let spawn_pos = target_position + Vec3::new(0.0, 1.0, 0.0);
                    if let Some(new_ent) = spawn_tile_instance(
                        &mut commands, &tile_models, tile, player_entity,
                        TileZone::Kawa, index, spawn_pos, target_rotation, tile_scale
                    ) {
                        commands.entity(new_ent).insert(AnimateTo { target_transform, speed: 10.0 });
                        busy.0 += 1;
                    }
                }

                visual_index += 1;
            }

            // Cleanup orphaned Kawa tiles (fixes the memory leak / round change bug)
            for (vis_entity, vis_slot, _) in existing_slots.iter() {
                if vis_slot.owner == player_entity
                    && vis_slot.zone == TileZone::Kawa
                    && !claimed_entities.contains(&vis_entity)
                {
                    commands.entity(vis_entity).despawn();
                }
            }
        }
    }
}


fn spawn_riichi_stick_system(
    mut commands: Commands,
    query: Query<(Entity, &Seat), Added<Riichi>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (player_entity, seat) in &query {
        let seat_index = seat.0;
        let seat_rotation = Quat::from_rotation_y(seat_index as f32 * std::f32::consts::FRAC_PI_2);

        let base_position = Vec3::new(0.0, 0.0, 0.7);
        let world_position = seat_rotation.mul_vec3(base_position);

        // rotate it 90 degrees so it lies horizontally in front of the player
        let stick_rotation = seat_rotation * Quat::from_rotation_y(std::f32::consts::PI);

        commands.spawn((
            VisualRiichiStick { owner: player_entity },
            Mesh3d(meshes.add(Cuboid::new(0.3, 0.02, 0.02))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.9, 0.9, 0.9),
                ..default()
            })),
            Transform {
                translation: world_position,
                rotation: stick_rotation,
                scale: Vec3::ONE,
            },
        ));
    }
}

fn cleanup_riichi_sticks_system(
    mut commands: Commands,
    sticks: Query<(Entity, &VisualRiichiStick)>,
    players: Query<&Riichi>,
) {
    for (stick_entity, stick) in &sticks {
        // despawn the stick if the riichi component is removed
        if players.get(stick.owner).is_err() {
            commands.entity(stick_entity).despawn();
        }
    }
}


// TODO: haipai https://www.youtube.com/watch?v=7BNe02MWLg0
// TODO: Kandora is broken
// TODO: yama tile to rinshan pile transfer for kan
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
        // 1. Skip standard drawn tiles
        if index < wall.head { continue; }

        // 2. Skip drawn rinshan tiles (matches the backwards array mapping)
        let mut was_rinshan_drawn = false;
        for r in 0..wall.rinshan_draws {
            let stack_offset = r / 2;
            let is_bot = r % 2 != 0;
            let base_stack_idx = wall.tiles.len() - 2 - (stack_offset * 2);
            let rinshan_idx = base_stack_idx + if is_bot { 1 } else { 0 };

            if index == rinshan_idx {
                was_rinshan_drawn = true;
                break;
            }
        }
        if was_rinshan_drawn { continue; }

        // 3. Native logical mapping (0 is the first draw, 67 is the Rinshan stack)
        let logical_stack = index / 2;
        let is_bottom = index % 2 != 0;

        // 4. Calculate where the table breaks based on dice roll
        // The break happens `dice_roll` stacks from the RIGHT edge.
        let break_side = (wall.oya_seat as usize + wall.dice_roll - 1) % 4;

        // 5. The first drawn tile is exactly to the left of the counted stacks.
        // If we count 11 from the right (stacks 16 down to 6), the first draw is stack 5.
        let first_draw_stack_in_side = 16 - wall.dice_roll;
        let first_draw_global = (break_side * 17) + first_draw_stack_in_side;

        // 6. Map logical_stack to physical_stack moving CLOCKWISE (subtracting instead of adding)
        // Add 68 before modulo to prevent negative numbers
        let physical_stack = (first_draw_global + 68 - logical_stack) % 68;

        let side = physical_stack / 17;
        let stack = physical_stack % 17;

        let side_rotation = Quat::from_rotation_y(side as f32 * std::f32::consts::FRAC_PI_2);

        // 7. Visual offset for the Wanpai (Dead Wall)
        // The Wanpai is the last 7 stacks. Since we draw clockwise, we need to push it
        // slightly counter-clockwise to create a visual gap from the end of the live wall.
        let is_dead_wall = logical_stack >= 61;
        let gap_offset = if is_dead_wall { -stack_spacing * 0.6 } else { 0.0 };

        let local_x = (stack as f32 - 8.0) * stack_spacing + gap_offset;
        let local_y = if is_bottom { 0.0 } else { tile_height };
        let local_z = wall_radius;

        let local_position = Vec3::new(local_x, local_y, local_z);
        let world_position = side_rotation.mul_vec3(local_position);

        // 8. Dora indicator check
        let mut is_dora = false;
        let rinshan_stacks = wall.rinshan_max / 2;
        for i in 0..wall.dora_count {
            let dora_idx = wall.tiles.len() - 2 - (rinshan_stacks + i) * 2;
            if index == dora_idx {
                is_dora = true;
                break;
            }
        }

        // 9. Final Rotations
        let placement_rotation = side_rotation * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
        let visual_rotation = if is_dora {
            placement_rotation * Quat::from_rotation_x(std::f32::consts::PI)
        } else {
            placement_rotation
        };

        // 10. Spawning
        let tile = &wall.tiles[index];
        let name = get_tile_model_name(tile);

        if let Some(parts) = tile_models.models.get(&name) {
            for part in parts {
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
                let (tiles, rot_idx) = match mentsu {
                    Mentsu::Jantou(t) => (t.as_slice(), None),
                    Mentsu::Koutsu(t, MentsuState::Open(idx)) => (t.as_slice(), Some(*idx)),
                    Mentsu::Koutsu(t, MentsuState::Closed) => (t.as_slice(), None),
                    Mentsu::Shuntsu(t, MentsuState::Open(idx)) => (t.as_slice(), Some(*idx)),
                    Mentsu::Shuntsu(t, MentsuState::Closed) => (t.as_slice(), None),
                    Mentsu::Ankan(t) => (t.as_slice(), None),
                    Mentsu::Daiminkan(t, idx) => (t.as_slice(), Some(*idx)),
                    Mentsu::Shouminkan(t, idx) => (t.as_slice(), Some(*idx)),
                };

                // decouple visual ordering from memory ordering
                let mut display_items: Vec<(usize, &Tile)> = tiles.iter().enumerate().collect();
                let mut visual_rot_idx = rot_idx;

                // chi is always from Kamicha, so we force the called tile to the visual left
                if let Mentsu::Shuntsu(_, MentsuState::Open(idx)) = mentsu {
                    let called = display_items.remove(*idx);
                    display_items.insert(0, called);
                    visual_rot_idx = Some(0);
                }

                let mut saved_kan_x = 0.0;

                for (visual_i, (orig_i, tile)) in display_items.into_iter().enumerate() {
                    let is_added_kan = matches!(mentsu, Mentsu::Shouminkan(..)) && orig_i == 3;
                    let is_rotated = Some(visual_i) == visual_rot_idx || is_added_kan;
                    let is_face_down = matches!(mentsu, Mentsu::Ankan(_)) && (orig_i == 0 || orig_i == 3);

                    let mut local_x = current_offset_x;
                    let mut local_z = 0.0; // use Z for depth

                    if is_added_kan {
                        local_x = saved_kan_x;
                        local_z = -0.19; // Shift forward into the table
                    } else if is_rotated {
                        current_offset_x += spacing_x * 0.15;
                        local_x = current_offset_x;
                        saved_kan_x = current_offset_x;
                    }

                    // Y remains 0.0 so it rests flat, Z controls depth
                    let local_position = Vec3::new(local_x, 0.0, local_z);
                    let world_position = seat_rotation.mul_vec3(base_position + local_position);

                    let tile_rotation = if is_rotated {
                        flat_rotation * Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)
                    } else if is_face_down {
                        flat_rotation * Quat::from_rotation_x(std::f32::consts::PI)
                    } else {
                        flat_rotation
                    };

                    spawn_mentsu_instance(&mut commands, &tile_models, tile, player_entity, world_position, tile_rotation, tile_scale);

                    if !is_added_kan {
                        current_offset_x += spacing_x;
                        if is_rotated {
                            current_offset_x += spacing_x * 0.3;
                        }
                    }
                }
                current_offset_x += 0.1;
            }
        }
    }
}


fn render_nukidora_system(
    mut commands: Commands,
    players_query: Query<(Entity, &NukedTiles, &Seat, Ref<NukedTiles>)>,
    tile_models: Res<TileModels>,
    existing_nuki_tiles: Query<(Entity, &VisualNukidoraTile)>,
) {
    if tile_models.models.is_empty() { return; }
    let force_redraw = tile_models.is_changed();

    for (player_entity, nuked, seat, ref_nuked) in &players_query {
        if force_redraw || ref_nuked.is_changed() {
            for (visual_entity, visual_tile) in &existing_nuki_tiles {
                if visual_tile.owner == player_entity {
                    commands.entity(visual_entity).despawn();
                }
            }

            let seat_index = seat.0;
            let seat_rotation = Quat::from_rotation_y(seat_index as f32 * std::f32::consts::FRAC_PI_2);

            let spacing_x = 0.19;
            let tile_scale = 0.015;

            // position to the right of the kawa
            let base_position = Vec3::new(1.0, 0.0, 1.0);
            let flat_rotation = seat_rotation * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);

            for (index, tile) in nuked.0.iter().enumerate() {
                let offset_x = index as f32 * spacing_x;
                let local_position = base_position + Vec3::new(offset_x, 0.0, 0.0);
                let world_position = seat_rotation.mul_vec3(local_position);

                let name = get_tile_model_name(tile);
                if let Some(parts) = tile_models.models.get(&name) {
                    for part in parts {
                        let part_transform = Transform {
                            translation: world_position + flat_rotation.mul_vec3(part.transform.translation * tile_scale),
                            rotation: flat_rotation * part.transform.rotation,
                            scale: Vec3::splat(tile_scale) * part.transform.scale,
                        };

                        commands.spawn((
                            VisualNukidoraTile { owner: player_entity },
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
    }
}


fn spawn_tile_instance(
    commands: &mut Commands,
    tile_models: &TileModels,
    tile: &Tile,
    owner: Entity,
    zone: TileZone,      // NEW
    index: usize,        // NEW
    position: Vec3,
    rotation: Quat,
    scale: f32,
) -> Option<Entity> {    // Return the entity so we can attach AnimateTo if needed
    let name = get_tile_model_name(tile);
    let parts = tile_models.models.get(&name)?;

    // We spawn a parent entity to hold the TileSlot, and attach the meshes as children.
    // This makes it much easier to animate one Transform instead of multiple mesh parts.
    let parent = commands.spawn((
        TileSlot { owner, zone, index, tile: *tile },
        Transform::from_translation(position).with_rotation(rotation),
        Visibility::default(),
        InheritedVisibility::default(),
    )).id();

    commands.entity(parent).with_children(|parent| {
        for part in parts {
            let part_transform = Transform {
                translation: part.transform.translation * scale,
                rotation: part.transform.rotation,
                scale: Vec3::splat(scale) * part.transform.scale,
            };

            parent.spawn((
                Mesh3d(part.mesh.clone()),
                MeshMaterial3d(part.material.clone()),
                part_transform,
            ));
        }
    });

    Some(parent)
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

pub fn cleanup_orphaned_visuals_system(
    mut commands: Commands,
    visual_slots: Query<(Entity, &TileSlot)>,
    visual_mentsu: Query<(Entity, &VisualMentsuTile)>,
    visual_nuki: Query<(Entity, &VisualNukidoraTile)>,
    players_query: Query<&Hand>,
) {
    for (visual_entity, slot) in &visual_slots {
        if players_query.get(slot.owner).is_err() {
            commands.entity(visual_entity).despawn();
        }
    }
    for (visual_entity, visual_tile) in &visual_mentsu {
        if players_query.get(visual_tile.owner).is_err() {
            commands.entity(visual_entity).despawn();
        }
    }
    for (visual_entity, visual_tile) in &visual_nuki {
        if players_query.get(visual_tile.owner).is_err() {
            commands.entity(visual_entity).despawn();
        }
    }
}


pub fn clear_board_visuals_system(
    mut commands: Commands,
    visual_slots: Query<Entity, With<TileSlot>>,
    visual_mentsu: Query<Entity, With<VisualMentsuTile>>,
    visual_nuki: Query<Entity, With<VisualNukidoraTile>>,
) {
    for entity in &visual_slots {
        commands.entity(entity).despawn();
    }
    for entity in &visual_mentsu {
        commands.entity(entity).despawn();
    }
    for entity in &visual_nuki {
        commands.entity(entity).despawn();
    }
}


pub fn sync_animation_busy_system(
    animating: Query<(), With<AnimateTo>>,
    mut busy: ResMut<AnimationBusy>,
) {
    // overwrite the manual counter with the actual physical reality of the board
    busy.0 = if animating.is_empty() { 0 } else { 1 };
}


pub fn process_animations_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &AnimateTo)>,
    mut busy: ResMut<AnimationBusy>,
) {
    let dt = time.delta_secs();

    for (entity, mut transform, anim) in query.iter_mut() {
        // Lerp translation
        transform.translation = transform.translation.lerp(anim.target_transform.translation, anim.speed * dt);

        // Slerp rotation
        transform.rotation = transform.rotation.slerp(anim.target_transform.rotation, anim.speed * dt);

        // Check if we arrived (using a small distance threshold)
        if transform.translation.distance(anim.target_transform.translation) < 0.005 {
            // Snap to exact target to avoid floating point drift
            transform.translation = anim.target_transform.translation;
            transform.rotation = anim.target_transform.rotation;

            // Remove the animation component
            commands.entity(entity).remove::<AnimateTo>();

            // Decrement the global busy counter
            if busy.0 > 0 {
                busy.0 -= 1;
            }
        }
    }
}