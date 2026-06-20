// /src/bin/showcase.rs

use bevy::prelude::*;
use bevy::gltf::{Gltf, GltfNode, GltfMesh};

struct ShowcasePlugin;

impl Plugin for ShowcasePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_camera, start_loading_gltf));
        app.add_systems(Update, check_and_spawn_showcase);
    }
}

#[derive(Resource)]
struct GltfLoadState {
    handle: Handle<Gltf>,
}

struct TilePart {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    transform: Transform,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ShowcasePlugin)
        .run();
}

fn setup_camera(mut commands: Commands) {
    // spawn a 3d camera overlooking the origin
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.5, 2.5).looking_at(Vec3::new(0.0, 0.0, -0.5), Vec3::Y),
    ));

    // light source to illuminate the tiles
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
    // load the entire gltf asset container instead of scene0
    let gltf_handle = asset_server.load("models/riichi_mahjong.glb");
    commands.insert_resource(GltfLoadState { handle: gltf_handle });
}

// recursive helper to collect all meshes and materials inside a gltf node hierarchy, preserving relative transforms
fn collect_tile_parts(
    node_handle: &Handle<GltfNode>,
    gltf_nodes: &Assets<GltfNode>,
    gltf_meshes: &Assets<GltfMesh>,
    accumulated_transform: Transform,
    out_list: &mut Vec<TilePart>,
    is_root: bool,
) {
    let Some(node) = gltf_nodes.get(node_handle) else { return; };

    // ignore the root node's own translation/rotation/scale because they are scene placement values
    let local_transform = if is_root {
        Transform::IDENTITY
    } else {
        node.transform
    };

    // combine the parent transform with this node's local transform
    let current_transform = accumulated_transform * local_transform;

    // check if this node contains a mesh
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

    // search children recursively
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

fn check_and_spawn_showcase(
    mut commands: Commands,
    load_state: Option<Res<GltfLoadState>>,
    gltf_assets: Res<Assets<Gltf>>,
    gltf_nodes: Res<Assets<GltfNode>>,
    gltf_meshes: Res<Assets<GltfMesh>>,
) {
    let Some(state) = load_state else { return };

    // check if gltf asset is fully loaded
    let Some(gltf) = gltf_assets.get(&state.handle) else {
        return;
    };

    // list of all unique tile name keys to look up in the gltf named_nodes map
    let tile_names = vec![
        "man1", "man2", "man3", "man4", "man5", "man6", "man7", "man8", "man9",
        "pin1", "pin2", "pin3", "pin4", "pin5", "pin6", "pin7", "pin8", "pin9",
        "sou1", "sou2", "sou3", "sou4", "sou5", "sou6", "sou7", "sou8", "sou9",
        "east", "south", "west", "north",
        "red_dragon", "green_dragon", "white_dragon",
    ];

    let mut spawned_count: f32 = 0.0;
    let columns = 9.0;

    // adjusted layout metrics for scaled down tiles
    let tile_scale = 0.02;
    let spacing_x = 0.35;
    let spacing_z = 0.5;

    for name in &tile_names {
        // try to find exact node name, or fall back to any node starting with the name
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
                let col_index = spawned_count % columns;
                let row_index = (spawned_count / columns).floor();

                let position_x = col_index * spacing_x - ((columns - 1.0) * spacing_x / 2.0);
                let position_z = -row_index * spacing_z;

                // spawn a parent anchor entity at the grid position
                let parent_entity = commands.spawn((
                    Transform {
                        translation: Vec3::new(position_x, 0.0, position_z),
                        rotation: Quat::IDENTITY,
                        scale: Vec3::splat(tile_scale),
                    },
                    Visibility::default(),
                    InheritedVisibility::default(),
                )).id();

                // spawn parts as children, maintaining their offsets relative to the parent
                for part in parts {
                    let child_entity = commands.spawn((
                        Mesh3d(part.mesh),
                        MeshMaterial3d(part.material),
                        part.transform,
                    )).id();
                    commands.entity(parent_entity).add_child(child_entity);
                }

                spawned_count += 1.0;
            } else {
                println!("Warning: Could not extract mesh/material from node: {}", name);
            }
        } else {
            println!("Warning: Node not found in GLTF: {}", name);
        }
    }

    println!("Spawned {} unique tile models.", spawned_count);

    // remove load state resource so this runs only once
    commands.remove_resource::<GltfLoadState>();
}
