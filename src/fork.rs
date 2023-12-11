use std::f32::consts::PI;

use bevy::{
    math::{vec2, vec3},
    prelude::*,
};
use bevy_ecs_ldtk::{
    prelude::*,
    utils::{grid_coords_to_translation, ldtk_grid_coords_to_grid_coords},
    EntityInstance, GridCoords, LevelIid,
};
use bevy_rapier2d::prelude::*;

use crate::{
    draggable::{drag_cancel_request, draggable_spawner, validate_drag, DragState, ValidDrag},
    game::GameState,
    inventory::Inventory,
    levels::{LevelLoadedEvent, LevelSize, NoPlacingHere},
    load::TextureAssets,
    mouse::{ClickSensor, ClickSensorEvent, Drag, DragDropConfirm, DragDropRequest, DragPos},
    physics::{coll_groups, ObjectGroup, Team},
    robot::{EngineDir, Robot},
};

pub struct ForkPlugin;

impl Plugin for ForkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (
                    draggable_spawner::<DraggedFork>("fork.png"),
                    validate_drag::<DraggedFork>,
                    drag_cancel_request::<DraggedFork>,
                ),
                check_click,
                drop_request,
                update_robot_motors,
                fixup_enemy_fork,
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Component, Default)]
struct LdtkDir(IVec2);

#[derive(Bundle, LdtkEntity, Default)]
pub struct EnemyForkBundle {
    #[with(extract_ldtk_dir)]
    ltdk_dir: LdtkDir,
    #[grid_coords]
    grid_coords: GridCoords,
    enemy_fork: EnemyFork,
}

fn extract_ldtk_dir(entity_instance: &EntityInstance) -> LdtkDir {
    LdtkDir(*entity_instance.get_point_field("direction").unwrap())
}

#[derive(Component, Default)]
struct EnemyFork;

fn fixup_enemy_fork(
    mut cmd: Commands,
    q_fork: Query<(Entity, &LdtkDir, &GridCoords, &Transform), With<EnemyFork>>,
    q_level: Query<Entity, With<LevelIid>>,
    level_size: Res<LevelSize>,
    mut ev_level_loaded: EventReader<LevelLoadedEvent>,
) {
    for _ in ev_level_loaded.read() {
        for (entity, LdtkDir(ldtk_dir), grid_coords, tr) in &q_fork {
            if let Some(level_size) = level_size.0 {
                let dir = ldtk_grid_coords_to_grid_coords(*ldtk_dir, level_size.size.y);
                let dir = grid_coords_to_translation(dir, level_size.tile_size_vec());
                let level_entity = q_level.single();
                cmd.entity(entity).remove::<LdtkDir>();
                let arrow = spawn_fork(
                    &mut cmd,
                    *tr,
                    (dir - tr.translation.truncate()).normalize(),
                    Team::Enemy,
                    None,
                    *grid_coords,
                );
                cmd.entity(level_entity).add_child(arrow);
            }
        }
    }
}

#[derive(Component)]
pub struct DraggedFork;

#[derive(Component)]
struct ForkClickSensor;

#[derive(Component)]
struct ForkRobotSensor;

#[derive(Component)]
pub struct Fork {
    dirs: [Vec2; 2],
    forked_count: usize,
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
            Collider::ball(96.),
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
