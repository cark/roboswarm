use bevy::{prelude::*, window::PrimaryWindow};
use bevy_ecs_ldtk::prelude::*;

use crate::{game::GameState, levels::LevelLoadedEvent};

// const CAMERA_SPEED: f32 = 100.0;

const CAMERA_DEFAULT_SCALE: f32 = 0.25;
const CAMERA_MIN_SCALE: f32 = 0.2;
const CAMERA_MAX_SCALE: f32 = 0.45;
pub struct GameCameraPlugin;

#[derive(Resource, Default)]
pub struct MouseWorldCoords(pub Option<Vec2>);

#[derive(Resource, Default)]
pub struct MouseScreenCoords(pub Option<Vec2>);

impl Plugin for GameCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init)
            .insert_resource(MouseWorldCoords::default())
            .insert_resource(MouseScreenCoords::default())
            .insert_resource(CameraTargetPos(Vec3::ZERO))
            .insert_resource(CameraTargetScale::new())
            //maybe we'll have to put fixup_camera_start in a preupdate
            .add_systems(Update, (update_mouse_coords, fixup_camera_start))
            .add_systems(
                PostUpdate,
                (move_camera, scale_camera).run_if(in_state(GameState::Playing)),
            );
        // app.add_systems(Update, bleh.run_if(in_state(GameState::Playing)));
    }
}

#[derive(Component)]
pub struct MainCamera;

#[derive(Resource, Default)]
pub struct CameraTargetPos(pub Vec3);

#[derive(Resource)]
pub struct CameraTargetScale(f32);

#[derive(Component, Default)]
pub struct CameraStart;

#[derive(Bundle, LdtkEntity, Default)]
pub struct CameraStartBundle {
    camera_start: CameraStart,
    #[grid_coords]
    grid_coords: GridCoords,
}

// fn bleh(q_level: Query<&GlobalTransform, With<LevelIid>>) {
//     for gtr in &q_level {
//         println!("{}", gtr.translation());
//     }
// }

fn fixup_camera_start(
    mut cmd: Commands,
    mut ev_level_loaded: EventReader<LevelLoadedEvent>,
    q_camera_start: Query<(Entity, &GlobalTransform), With<CameraStart>>,
    //q_level: Query<Entity, (With<LevelIid>, Changed<Transform>)>,
    mut camera_target_pos: ResMut<CameraTargetPos>,
    mut camera_target_scale: ResMut<CameraTargetScale>,
) {
    for _ in ev_level_loaded.read() {
        //for _ in &q_level {
        for (entity, gtr) in &q_camera_start {
            info!(
                "fixup camera. camera start global translation: {}",
                gtr.translation()
            );
            //println!("{}", gtr.translation());
            //println!("{}", q_level.single().translation());
            camera_target_pos.0 = gtr.translation();
            camera_target_scale.0 = CAMERA_DEFAULT_SCALE;
            cmd.entity(entity).despawn_recursive();
        }
    }
}

impl CameraTargetScale {
    pub fn new() -> Self {
        Self(CAMERA_DEFAULT_SCALE)
    }

    pub fn get(&self) -> f32 {
        self.0
    }

    pub fn set(&mut self, value: f32) -> f32 {
        let value = value.clamp(CAMERA_MIN_SCALE, CAMERA_MAX_SCALE);
        self.0 = value;
        value
    }
}

fn init(mut cmd: Commands) {
    let mut bundle = Camera2dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..Default::default()
    };
    bundle.projection.scale = CAMERA_DEFAULT_SCALE;
    cmd.spawn((MainCamera, bundle));
}

fn update_mouse_coords(
    mut mouse_world_coords: ResMut<MouseWorldCoords>,
    mut mouse_screen_coords: ResMut<MouseScreenCoords>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    // get the camera info and transform
    // assuming there is exactly one main camera entity, so Query::single() is OK
    let (camera, camera_transform) = q_camera.single();

    // There is only one primary window, so we can similarly get it from the query:
    let window = q_window.single();

    // check if the cursor is inside the window and get its position
    // then, ask bevy to convert into world coordinates, and truncate to discard Z
    mouse_world_coords.0 = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate());
    mouse_screen_coords.0 = window.cursor_position();
}

fn move_camera(
    mut q_camera: Query<&mut Transform, With<MainCamera>>,
    camera_target_pos: Res<CameraTargetPos>,
    // time: Res<Time>,
) {
    for mut tr in q_camera.iter_mut() {
        let d_pos = camera_target_pos.0 - tr.translation;
        // let speed = delta_d / time.
        let new_loc = tr.translation + d_pos / 10.0;
        tr.translation = new_loc;
    }
}

fn scale_camera(
    mut q_camera: Query<&mut OrthographicProjection, With<MainCamera>>,
    target_scale: Res<CameraTargetScale>,
) {
    for mut projection in &mut q_camera {
        projection.scale = projection.scale + (target_scale.0 - projection.scale) / 10.0;
    }
}
