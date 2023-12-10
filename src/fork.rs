use std::f32::consts::PI;

use bevy::{
    math::{vec2, vec3},
    prelude::*,
    utils::info,
};
use bevy_ecs_ldtk::{
    utils::{grid_coords_to_translation, translation_to_grid_coords},
    GridCoords, LevelIid,
};
use bevy_rapier2d::prelude::*;

use crate::{
    game::GameState,
    game_camera::MouseWorldCoords,
    inventory::Inventory,
    levels::{LevelSize, NoPlacingHere, WallCache},
    load::TextureAssets,
    mouse::{
        ClickSensor, ClickSensorEvent, Drag, DragCancelConfirm, DragCancelRequest, DragDropConfirm,
        DragDropRequest, DragPos,
    },
    physics::{coll_groups, ObjectGroup, Team},
    portal::Portal,
    robot::{EngineDir, Robot},
};

pub struct ForkPlugin;

impl Plugin for ForkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_draggable_fork,
                validate_drag,
                check_click,
                (drag_cancel_request, drop_request).chain(),
                update_robot_motors,
                // fixup_enemy_arrow,
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Component)]
pub struct DraggedFork;

#[derive(Component)]
struct ForkClickSensor;

#[derive(Component)]
struct ForkRobotSensor;

#[derive(Component, PartialEq)]
enum DragState {
    Dragging,
    SettingDirection(Transform),
}

#[derive(Component)]
pub struct Fork {
    dirs: [Vec2; 2],
    forked_count: usize,
}

#[derive(Component)]
struct ValidDrag;

fn spawn_draggable_fork(
    mut cmd: Commands,
    assets: Res<TextureAssets>,
    q_drag: Query<(Entity, &DragPos), (With<DraggedFork>, Added<Drag>)>,
) {
    for (entity, drag_pos) in &q_drag {
        cmd.entity(entity).insert((
            SpriteBundle {
                transform: Transform::from_translation(drag_pos.0.extend(0.0)),
                texture: assets.fork.clone(),
                ..Default::default()
            },
            DragState::Dragging,
        ));
    }
}

fn validate_drag(
    mut cmd: Commands,
    mut q_drag: Query<(Entity, &mut Transform, &mut Sprite, &DragState), With<DraggedFork>>,
    mouse_pos: Res<MouseWorldCoords>,
    q_level: Query<(&GlobalTransform, &WallCache), With<LevelIid>>,
    level_size: Res<LevelSize>,
    q_occupied: Query<&GridCoords, With<NoPlacingHere>>, //wall_cache: Res<WallCache>,
) {
    for (entity, mut drag_tr, mut sprite, drag_state) in &mut q_drag {
        match drag_state {
            DragState::Dragging => {
                cmd.entity(entity).remove::<ValidDrag>();
                if let Some(pos) = mouse_pos.0 {
                    drag_tr.translation = pos.extend(0.0);
                    sprite.color = Color::WHITE.with_a(0.4);
                    let (level_gtr, wall_cache) = q_level.single();
                    if let Some(size_info) = level_size.0 {
                        let coords = translation_to_grid_coords(
                            drag_tr.translation.truncate() - level_gtr.translation().truncate(),
                            size_info.tile_size_vec(),
                        );
                        if size_info.grid_coords_in_bound(coords)
                            && !wall_cache.items.contains_key(&coords)
                            && q_occupied.iter().all(|grid_coord| {
                                (Into::<IVec2>::into(*grid_coord) - Into::<IVec2>::into(coords))
                                    .as_vec2()
                                    .length()
                                    >= 2.0
                            })
                        {
                            drag_tr.translation =
                                grid_coords_to_translation(coords, size_info.tile_size_vec())
                                    .extend(0.0)
                                    + level_gtr
                                        .translation()
                                        .truncate()
                                        .extend(drag_tr.translation.z);
                            sprite.color = Color::WHITE.with_a(1.0);
                            cmd.entity(entity).insert(ValidDrag).insert(coords);
                        }
                    }
                }
            }
            DragState::SettingDirection(center_tr) => {
                if let Some(pos) = mouse_pos.0 {
                    let angle =
                        vec2(1.0, 0.0).angle_between(pos - center_tr.translation.truncate());
                    *drag_tr = drag_tr.with_rotation(Quat::from_rotation_z(angle));
                }
            }
        }
    }
}

fn drag_cancel_request(
    mut cmd: Commands,
    q_drag: Query<Entity, (Added<DragCancelRequest>, With<DraggedFork>)>,
) {
    for entity in &q_drag {
        cmd.entity(entity).insert(DragCancelConfirm);
    }
}

fn drop_request(
    mut cmd: Commands,
    mut q_drag: Query<
        (
            Entity,
            &Transform,
            Option<&ValidDrag>,
            &mut DragState,
            &GridCoords,
        ),
        (With<DragDropRequest>, With<DraggedFork>),
    >,
    assets: Res<TextureAssets>,
    mut inventory: ResMut<Inventory>,
    q_level: Query<(Entity, &GlobalTransform), With<LevelIid>>,
) {
    for (entity, drag_tr, maybe_valid, mut drag_state, grid_coords) in &mut q_drag {
        if maybe_valid.is_some() {
            match *drag_state {
                DragState::Dragging => {
                    *drag_state = DragState::SettingDirection(*drag_tr);
                    cmd.entity(entity).remove::<DragDropRequest>();
                }
                DragState::SettingDirection(_) => {
                    let dir = drag_tr.rotation.mul_vec3(vec3(1.0, 0.0, 0.0));
                    let (level_entity, level_gtr) = q_level.single();
                    inventory.fork_count -= 1;
                    cmd.entity(entity).insert(DragDropConfirm);
                    let local_pos = drag_tr.translation - level_gtr.translation();
                    //info(grid_coords);
                    let fork = spawn_fork(
                        &mut cmd,
                        drag_tr.with_translation(local_pos),
                        dir.truncate(),
                        Team::Player,
                        Some(assets.fork.clone()),
                        *grid_coords,
                    );
                    cmd.entity(level_entity).add_child(fork);
                }
            }
        } else {
            cmd.entity(entity).remove::<DragDropRequest>();
        }
    }
}

fn spawn_fork(
    cmd: &mut Commands,
    tr: Transform,
    dir: Vec2,
    team: Team,
    texture: Option<Handle<Image>>,
    grid_coords: GridCoords,
) -> Entity {
    let dirs = [
        Quat::from_rotation_z(PI / 4.)
            .mul_vec3(dir.extend(0.0))
            .truncate(),
        Quat::from_rotation_z(-PI / 4.)
            .mul_vec3(dir.extend(0.0))
            .truncate(),
    ];
    let fork_entity = cmd
        .spawn((
            Fork {
                dirs,
                forked_count: 0,
            },
            team,
            grid_coords,
        ))
        .id();
    if team == Team::Player {
        cmd.entity(fork_entity).insert((
            SpriteBundle {
                texture: texture.unwrap(),
                transform: tr,
                ..Default::default()
            },
            NoPlacingHere,
        ));
    } else {
        cmd.entity(fork_entity)
            .insert(TransformBundle::from_transform(tr));
    }
    cmd.entity(fork_entity).with_children(|cmd| {
        if team == Team::Player {
            cmd.spawn((
                ForkClickSensor,
                ClickSensor,
                Collider::capsule(vec2(-4., 0.0), vec2(3., 0.0), 12.),
                Sensor,
                TransformBundle::default(),
            ));
        }
        cmd.spawn((
            ForkRobotSensor,
            Sensor,
            Collider::ball(if team == Team::Player { 96. } else { 96. * 2. }),
            match team {
                Team::Player => {
                    coll_groups(ObjectGroup::PLAYER_FORK_SENSOR, ObjectGroup::PLAYER_ROBOT)
                }
                Team::Enemy => {
                    coll_groups(ObjectGroup::ENEMY_FORK_SENSOR, ObjectGroup::ENEMY_ROBOT)
                }
            },
            TransformBundle::default(),
            ActiveEvents::COLLISION_EVENTS,
        ));
    });
    fork_entity
}

fn check_click(
    mut cmd: Commands,
    mut ev_click_sensor: EventReader<ClickSensorEvent>,
    q_sensor: Query<&Parent, With<ForkClickSensor>>,
    q_fork: Query<(Entity, &GlobalTransform, &Team), With<Fork>>,
    mut inventory: ResMut<Inventory>,
) {
    for ClickSensorEvent(sensor_entity) in ev_click_sensor.read() {
        if let Ok((fork_entity, fork_gtr, team)) = q_sensor
            .get(*sensor_entity)
            .map(|parent| parent.get())
            .and_then(|fork_entity| q_fork.get(fork_entity))
        {
            if *team == Team::Player {
                inventory.fork_count += 1;
                cmd.spawn((
                    Drag,
                    DragPos(fork_gtr.translation().truncate()),
                    DraggedFork,
                ));
                cmd.entity(fork_entity).despawn_recursive();
            }
        }
    }
}

fn update_robot_motors(
    mut collision_events: EventReader<CollisionEvent>,
    q_robot_sensor: Query<&Parent, With<ForkRobotSensor>>,
    mut q_fork: Query<(&mut Fork, &Team)>,
    mut q_robot: Query<(&mut EngineDir, &Team), With<Robot>>,
) {
    for ev in collision_events.read() {
        if let CollisionEvent::Started(e1, e2, _) = ev {
            let (sensor, other) = match (q_robot_sensor.contains(*e1), q_robot_sensor.contains(*e2))
            {
                (true, false) => (*e1, *e2),
                (false, true) => (*e2, *e1),
                _ => continue,
            };
            if let (Ok((mut engine_dir, robot_team)), Ok((mut fork, fork_team))) = (
                q_robot.get_mut(other),
                q_robot_sensor
                    .get(sensor)
                    .and_then(|parent| q_fork.get_mut(parent.get())),
            ) {
                if robot_team == fork_team {
                    engine_dir.0 = fork.dirs[fork.forked_count % 2];
                    fork.forked_count += 1;
                }
            }
        }
    }
}
