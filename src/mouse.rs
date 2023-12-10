use bevy::{
    input::mouse::{MouseMotion, MouseWheel},
    math::{vec2, vec3},
    prelude::*,
    window::CursorGrabMode,
};
use bevy_rapier2d::{pipeline::QueryFilter, plugin::RapierContext};

use crate::{
    game::GameState,
    game_camera::{CameraTargetPos, CameraTargetScale, MainCamera, MouseWorldCoords},
};

pub struct MousePlugin;

#[derive(Default, Resource, PartialEq, Clone, Copy)]
pub enum MouseState {
    #[default]
    Default,
    CameraMovement,
    Dragging,
}

impl Plugin for MousePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MouseState::default())
            // .init_resource::<DragInfo>()
            // .add_event::<StartDragEvent>()
            // .add_event::<CancelDragEvent>()
            .add_event::<ClickSensorEvent>()
            .add_systems(
                Update,
                (motion, buttons, scroll_wheel, check_for_click_sensor)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
        app.add_systems(
            PreUpdate,
            (start_drag, drag_cancel_confirm, drag_drop_confirm),
        );
    }
}

#[derive(Component, Default)]
pub struct Drag;

// #[derive(Component)]
// pub struct DragTarget(pub Entity);

#[derive(Component)]
pub struct DragPos(pub Vec2);

#[derive(Component)]
pub struct DragCancelRequest;

#[derive(Component)]
pub struct DragCancelConfirm;

#[derive(Component)]
pub struct DragDropRequest;

#[derive(Component)]
pub struct DragDropConfirm;

#[derive(Component)]
pub struct ClickSensor;

#[derive(Event)]
pub struct ClickSensorEvent(pub Entity);

fn start_drag(q_drag: Query<Entity, Added<Drag>>, mut mouse_state: ResMut<MouseState>) {
    for _ in &q_drag {
        *mouse_state = MouseState::Dragging;
    }
}

fn drag_cancel_confirm(
    mut cmd: Commands,
    q_drag: Query<Entity, (With<DragCancelRequest>, With<DragCancelConfirm>)>,
    mut mouse_state: ResMut<MouseState>,
) {
    for entity in &q_drag {
        cmd.entity(entity).despawn_recursive();
        *mouse_state = MouseState::Default;
    }
}

fn drag_drop_confirm(
    mut cmd: Commands,
    q_drag: Query<Entity, (With<DragDropRequest>, With<DragDropConfirm>)>,
    mut mouse_state: ResMut<MouseState>,
) {
    for entity in &q_drag {
        cmd.entity(entity).despawn_recursive();
        *mouse_state = MouseState::Default;
    }
}

fn buttons(
    mut cmd: Commands,
    mut mouse_state: ResMut<MouseState>,
    button: Res<Input<MouseButton>>,
    mut windows: Query<&mut Window>,
    q_camera: Query<&Transform, With<MainCamera>>,
    mut target_pos: ResMut<CameraTargetPos>,
    q_drag: Query<Entity, With<Drag>>,
) {
    let ms = *mouse_state;
    match ms {
        MouseState::Default => {
            if button.pressed(MouseButton::Right) {
                *mouse_state = MouseState::CameraMovement;
                let mut window = windows.single_mut();
                window.cursor.visible = false;
                if cfg!(not(target_arch = "wasm32")) {
                    window.cursor.grab_mode = CursorGrabMode::Locked;
                }
                let tr = q_camera.single();
                target_pos.0 = tr.translation;
            }
        }
        MouseState::CameraMovement => {
            if !button.pressed(MouseButton::Right) {
                *mouse_state = MouseState::Default;
                let mut window = windows.single_mut();
                window.cursor.visible = true;
                if cfg!(not(target_arch = "wasm32")) {
                    window.cursor.grab_mode = CursorGrabMode::None;
                }
            }
        }
        MouseState::Dragging => {
            if button.just_pressed(MouseButton::Right) {
                for entity in &q_drag {
                    cmd.entity(entity).insert(DragCancelRequest);
                }
            } else if button.just_pressed(MouseButton::Left) {
                for entity in &q_drag {
                    cmd.entity(entity).insert(DragDropRequest);
                }
            }
        }
    }
}

pub fn motion(
    mut motion_ev: EventReader<MouseMotion>,
    mouse_state: ResMut<MouseState>,
    mut target_pos: ResMut<CameraTargetPos>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    for ev in motion_ev.read() {
        if *mouse_state == MouseState::CameraMovement {
            let (camera, camera_transform) = q_camera.single();
            let world_pos = target_pos.0.truncate();
            let window_pos = camera
                .world_to_ndc(camera_transform, world_pos.extend(0.0))
                .unwrap();

            let division = vec2(200., 150.);
            let division = if cfg!(target_arch = "wasm32") {
                division * 2.0
            } else {
                division
            };
            let target_window_pos =
                window_pos + vec3(-ev.delta.x / division.x, ev.delta.y / division.y, 0.0);
            let target_world_pos = camera
                .ndc_to_world(camera_transform, target_window_pos)
                .unwrap();
            target_pos.0 = target_world_pos;
        }
    }
}

fn scroll_wheel(
    mut scroll_ev: EventReader<MouseWheel>,
    mut target_scale: ResMut<CameraTargetScale>,
) {
    use bevy::input::mouse::MouseScrollUnit;
    for ev in scroll_ev.read() {
        let units = match ev.unit {
            MouseScrollUnit::Line => {
                // info!("Line");
                ev.y * 20.0
            }
            MouseScrollUnit::Pixel => {
                // info!("Pixel");
                ev.y / 5.
            }
        };
        let scale = target_scale.get();
        target_scale.set(scale * 1.01f32.powf(-units));
    }
}

fn check_for_click_sensor(
    q_click_sensor: Query<Entity, With<ClickSensor>>,
    mouse_state: Res<MouseState>,
    mouse_pos: Res<MouseWorldCoords>,
    button: Res<Input<MouseButton>>,
    rapier_context: Res<RapierContext>,
    mut ev_click_sensor: EventWriter<ClickSensorEvent>,
) {
    if *mouse_state == MouseState::Default && button.just_released(MouseButton::Left) {
        if let Some(mouse_pos) = mouse_pos.0 {
            let point = mouse_pos;
            let filter = QueryFilter::default();
            rapier_context.intersections_with_point(point, filter, |entity| {
                if q_click_sensor.contains(entity) {
                    ev_click_sensor.send(ClickSensorEvent(entity));
                }
                true
            });
        }
    }
}
