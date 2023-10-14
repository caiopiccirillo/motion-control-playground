//! A simple glTF scene viewer made with Bevy.
//!
//! Just run `cargo run --release /path/to/model.gltf`,
//! replacing the path as appropriate.
//! In case of multiple scenes, you can select which to display by adapting the file path: `/path/to/model.gltf#Scene1`.
//! With no arguments it will load the `rotary_pendulum` glTF model from the repository assets subdirectory.

use bevy::{
    asset::ChangeWatcher,
    math::Vec3A,
    prelude::*,
    render::primitives::{Aabb, Sphere},
    utils::Duration,
    window::WindowPlugin,
};

use bevy_infinite_grid::{InfiniteGrid, InfiniteGridBundle, InfiniteGridPlugin};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier3d::prelude::*;

mod scene_viewer_plugin;

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use scene_viewer_plugin::{SceneHandle, SceneViewerPlugin};

fn main() {
    let mut app = App::new();
    app.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 1.0 / 5.0f32,
    })
    .add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "bevy scene viewer".to_string(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                asset_folder: std::env::var("CARGO_MANIFEST_DIR")
                    .unwrap_or_else(|_| ".".to_string()),
                watch_for_changes: ChangeWatcher::with_delay(Duration::from_millis(200)),
            }),
        PanOrbitCameraPlugin,
        SceneViewerPlugin,
        WorldInspectorPlugin::new(),
        RapierPhysicsPlugin::<NoUserData>::default(),
        RapierDebugRenderPlugin::default(),
        InfiniteGridPlugin,
    ))
    .add_systems(Startup, setup)
    .add_systems(PreUpdate, setup_scene_after_load);

    app.run();
}

fn parse_scene(scene_path: String) -> (String, usize) {
    if scene_path.contains('#') {
        let gltf_and_scene = scene_path.split('#').collect::<Vec<_>>();
        if let Some((last, path)) = gltf_and_scene.split_last() {
            if let Some(index) = last
                .strip_prefix("Scene")
                .and_then(|index| index.parse::<usize>().ok())
            {
                return (path.join("#"), index);
            }
        }
    }
    (scene_path, 0)
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let scene_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/rotary_pendulum.glb".to_string());
    info!("Loading {}", scene_path);
    let (file_path, scene_index) = parse_scene(scene_path);
    commands.insert_resource(SceneHandle::new(asset_server.load(file_path), scene_index));
}

fn setup_scene_after_load(
    mut commands: Commands,
    mut setup: Local<bool>,
    mut scene_handle: ResMut<SceneHandle>,
    asset_server: Res<AssetServer>,
    meshes: Query<(&GlobalTransform, Option<&Aabb>), With<Handle<Mesh>>>,
) {
    if scene_handle.is_loaded && !*setup {
        *setup = true;
        // Find an approximate bounding box of the scene from its meshes
        if meshes.iter().any(|(_, maybe_aabb)| maybe_aabb.is_none()) {
            return;
        }

        let mut min = Vec3A::splat(f32::MAX);
        let mut max = Vec3A::splat(f32::MIN);
        for (transform, maybe_aabb) in &meshes {
            let aabb = maybe_aabb.unwrap();
            // If the Aabb had not been rotated, applying the non-uniform scale would produce the
            // correct bounds. However, it could very well be rotated and so we first convert to
            // a Sphere, and then back to an Aabb to find the conservative min and max points.
            let sphere = Sphere {
                center: Vec3A::from(transform.transform_point(Vec3::from(aabb.center))),
                radius: transform.radius_vec3a(aabb.half_extents),
            };
            let aabb = Aabb::from(sphere);
            min = min.min(aabb.min());
            max = max.max(aabb.max());
        }

        // Display the controls of the scene viewer
        info!("{}", *scene_handle);

        commands.spawn((
            Camera3dBundle {
                transform: Transform::from_translation(Vec3::new(10.0, 10.0, 10.0)),
                ..default()
            },
            PanOrbitCamera::default(),
            EnvironmentMapLight {
                diffuse_map: asset_server
                    .load("assets/environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
                specular_map: asset_server
                    .load("assets/environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            },
        ));

        // Spawn a default light if the scene does not have one
        if !scene_handle.has_light {
            info!("Spawning a directional light");
            commands.spawn(DirectionalLightBundle {
                directional_light: DirectionalLight {
                    shadows_enabled: false,
                    ..default()
                },
                ..default()
            });

            scene_handle.has_light = true;
        }

        commands.spawn(InfiniteGridBundle {
            grid: InfiniteGrid {
                // shadow_color: None,
                ..default()
            },
            ..default()
        });
    }
}
