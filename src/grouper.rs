use std::{collections::VecDeque, f32::consts::PI};

use bevy::{
    math::{vec2, vec3},
    prelude::*,
    utils::HashSet,
};
use bevy_ecs_ldtk::{
    prelude::*,
    utils::{grid_coords_to_translation, ldtk_grid_coords_to_grid_coords},
    GridCoords, LevelIid,
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

const GROUP_SIZE: usize = 25;
pub struct GrouperPlugin;

impl Plugin for GrouperPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (
                    draggable_spawner::<DraggedGrouper>("grouper.png"),
                    validate_drag::<DraggedGrouper>,
                    drag_cancel_request::<DraggedGrouper>,
                    drop_request,
                    check_click,
                ),
                update_robot_motors,
                fixup_enemy_grouper,
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Component, Default)]
struct LdtkDir(IVec2);

#[derive(Bundle, LdtkEntity, Default)]
pub struct EnemyGrouperBundle {
    #[with(extract_ldtk_dir)]
    ltdk_dir: LdtkDir,
    #[grid_coords]
    grid_coords: GridCoords,
    enemy_grouper: EnemyGrouper,
}

fn extract_ldtk_dir(entity_instance: &EntityInstance) -> LdtkDir {
    LdtkDir(*entity_instance.get_point_field("direction").unwrap())
}

#[derive(Component, Default)]
struct EnemyGrouper;

fn fixup_enemy_grouper(
    mut cmd: Commands,
    q_grouper: Query<(Entity, &LdtkDir, &GridCoords, &Transform), With<EnemyGrouper>>,
    q_level: Query<Entity, With<LevelIid>>,
    level_size: Res<LevelSize>,
    mut ev_level_loaded: EventReader<LevelLoadedEvent>,
) {
    for _ in ev_level_loaded.read() {
        for (entity, LdtkDir(ldtk_dir), grid_coords, tr) in &q_grouper {
            if let Some(level_size) = level_size.0 {
                let dir = ldtk_grid_coords_to_grid_coords(*ldtk_dir, level_size.size.y);
                let dir = grid_coords_to_translation(dir, level_size.tile_size_vec());
                let level_entity = q_level.single();
                cmd.entity(entity).remove::<LdtkDir>();
                let grouper = spawn_grouper(
                    &mut cmd,
                    *tr,
                    (dir - tr.translation.truncate()).normalize(),
                    Team::Enemy,
                    None,
                    *grid_coords,
                );
                cmd.entity(level_entity).add_child(grouper);
            }
        }
    }
}

#[derive(Component)]
pub struct DraggedGrouper;

#[derive(Component)]
struct GrouperClickSensor;

#[derive(Component)]
struct GrouperRobotSensor;

#[derive(Component)]
pub struct Grouper {
    dir: Vec2,
    group: HashSet<Entity>,
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
        (With<DragDropRequest>, With<DraggedGrouper>),
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
                    inventory.grouper_count -= 1;
                    cmd.entity(entity).insert(DragDropConfirm);
                    let local_pos = drag_tr.translation - level_gtr.translation();
                    //info(grid_coords);
                    let fork = spawn_grouper(
                        &mut cmd,
                        drag_tr.with_translation(local_pos),
                        dir.truncate(),
                        Team::Player,
                        Some(assets.grouper.clone()),
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

fn spawn_grouper(
    cmd: &mut Commands,
    tr: Transform,
    dir: Vec2,
    team: Team,
    texture: Option<Handle<Image>>,
    grid_coords: GridCoords,
) -> Entity {
    let spawned_entity = cmd
        .spawn((
            Grouper {
                dir,
                group: Default::default(),
            },
            team,
            grid_coords,
        ))
        .id();
    if team == Team::Player {
        cmd.entity(spawned_entity).insert((
            SpriteBundle {
                texture: texture.unwrap(),
                transform: tr,
                ..Default::default()
            },
            NoPlacingHere,
        ));
    } else {
        cmd.entity(spawned_entity)
            .insert(TransformBundle::from_transform(tr));
    }
    cmd.entity(spawned_entity).with_children(|cmd| {
        if team == Team::Player {
            cmd.spawn((
                GrouperClickSensor,
                ClickSensor,
                Collider::capsule(vec2(-4., 0.0), vec2(3., 0.0), 12.),
                Sensor,
                TransformBundle::default(),
            ));
        }
        cmd.spawn((
            GrouperRobotSensor,
            Sensor,
            Collider::ball(96.),
            match team {
                Team::Player => coll_groups(
                    ObjectGroup::PLAYER_GROUPER_SENSOR,
                    ObjectGroup::PLAYER_ROBOT,
                ),
                Team::Enemy => {
                    coll_groups(ObjectGroup::ENEMY_GROUPER_SENSOR, ObjectGroup::ENEMY_ROBOT)
                }
            },
            TransformBundle::default(),
            ActiveEvents::COLLISION_EVENTS,
        ));
    });
    spawned_entity
}

fn check_click(
    mut cmd: Commands,
    mut ev_click_sensor: EventReader<ClickSensorEvent>,
    q_sensor: Query<&Parent, With<GrouperClickSensor>>,
    q_grouper: Query<(Entity, &GlobalTransform, &Team), With<Grouper>>,
    mut inventory: ResMut<Inventory>,
) {
    for ClickSensorEvent(sensor_entity) in ev_click_sensor.read() {
        if let Ok((arrow_entity, arrow_gtr, team)) = q_sensor
            .get(*sensor_entity)
            .map(|parent| parent.get())
            .and_then(|arrow_entity| q_grouper.get(arrow_entity))
        {
            if *team == Team::Player {
                inventory.grouper_count += 1;
                cmd.spawn((
                    Drag,
                    DragPos(arrow_gtr.translation().truncate()),
                    DraggedGrouper,
                ));
                cmd.entity(arrow_entity).despawn_recursive();
            }
        }
    }
}

fn update_robot_motors(
    mut collision_events: EventReader<CollisionEvent>,
    q_robot_sensor: Query<&Parent, With<GrouperRobotSensor>>,
    mut q_grouper: Query<(&mut Grouper, &Team, &Transform)>,
    mut q_robot: Query<(&mut EngineDir, &Team, &Transform), With<Robot>>,
) {
    for ev in collision_events.read() {
        match ev {
            CollisionEvent::Started(e1, e2, _) => {
                let (sensor, other) =
                    match (q_robot_sensor.contains(*e1), q_robot_sensor.contains(*e2)) {
                        (true, false) => (*e1, *e2),
                        (false, true) => (*e2, *e1),
                        _ => continue,
                    };
                if let (Ok((_, robot_team, _)), Ok((mut grouper, grouper_team, _))) = (
                    q_robot.get(other),
                    q_robot_sensor
                        .get(sensor)
                        .and_then(|parent| q_grouper.get_mut(parent.get())),
                ) {
                    if robot_team == grouper_team {
                        grouper.group.insert(other);
                        if grouper.group.len() >= GROUP_SIZE {
                            for &entity in grouper.group.iter() {
                                if let Ok((mut engine_dir, _, _)) = q_robot.get_mut(entity) {
                                    engine_dir.0 = grouper.dir;
                                }
                            }
                            grouper.group.clear();
                        }
                    }
                }
            }
            CollisionEvent::Stopped(_e1, _e2, _ev_flags) => {}
        }
    }
    for (grouper, _, g_tr) in &q_grouper {
        for e_robot in grouper.group.iter() {
            if let Ok((mut engine_dir, _, r_tr)) = q_robot.get_mut(*e_robot) {
                let to_grouper = (g_tr.translation - r_tr.translation).normalize();
                let rotated = Quat::from_rotation_z(-PI / 4.0)
                    .mul_vec3(to_grouper)
                    .truncate();
                engine_dir.0 = rotated * 1.5; //to_grouper.truncate();
            }
        }
    }
}
