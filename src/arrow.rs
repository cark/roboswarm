use bevy::{
    math::{vec2, vec3},
    prelude::*,
};
use bevy_ecs_ldtk::{
    ldtk::ldtk_fields::LdtkFields,
    utils::{
        grid_coords_to_translation, ldtk_grid_coords_to_grid_coords, translation_to_grid_coords,
    },
    EntityInstance, GridCoords, LdtkEntity, LevelIid,
};
use bevy_rapier2d::{
    geometry::{ActiveEvents, Collider, Sensor},
    pipeline::CollisionEvent,
};

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

pub struct ArrowPlugin;

impl Plugin for ArrowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (
                    draggable_spawner::<DraggedArrow>("arrow.png"),
                    validate_drag::<DraggedArrow>,
                    drag_cancel_request::<DraggedArrow>,
                ),
                check_click,
                (drop_request).chain(),
                update_robot_motors,
                fixup_enemy_arrow,
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Component, Default)]
struct LdtkDir(IVec2);

#[derive(Bundle, LdtkEntity, Default)]
pub struct EnemyArrowBundle {
    #[with(extract_ldtk_dir)]
    ltdk_dir: LdtkDir,
    #[grid_coords]
    grid_coords: GridCoords,
    enemy_arrow: EnemyArrow,
}

#[derive(Component, Default)]
struct EnemyArrow;

fn extract_ldtk_dir(entity_instance: &EntityInstance) -> LdtkDir {
    LdtkDir(*entity_instance.get_point_field("direction").unwrap())
}

#[derive(Component)]
pub struct DraggedArrow;

#[derive(Component)]
pub struct Arrow {
    dir: Vec2,
}

#[derive(Component)]
struct ArrowClickSensor;

#[derive(Component)]
struct ArrowRobotSensor;

fn fixup_enemy_arrow(
    mut cmd: Commands,
    q_arrow: Query<(Entity, &LdtkDir, &GridCoords, &Transform), With<EnemyArrow>>,
    q_level: Query<Entity, With<LevelIid>>,
    level_size: Res<LevelSize>,
    mut ev_level_loaded: EventReader<LevelLoadedEvent>,
) {
    for _ in ev_level_loaded.read() {
        for (entity, LdtkDir(ldtk_dir), grid_coords, tr) in &q_arrow {
            if let Some(level_size) = level_size.0 {
                let dir = ldtk_grid_coords_to_grid_coords(*ldtk_dir, level_size.size.y);
                let dir = grid_coords_to_translation(dir, level_size.tile_size_vec());
                let level_entity = q_level.single();
                cmd.entity(entity).remove::<LdtkDir>();
                let arrow = spawn_arrow(
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

fn spawn_arrow(
    cmd: &mut Commands,
    tr: Transform,
    dir: Vec2,
    team: Team,
    texture: Option<Handle<Image>>,
    grid_coords: GridCoords,
) -> Entity {
    let arrow_entity = cmd.spawn((Arrow { dir }, team, grid_coords)).id();
    if team == Team::Player {
        cmd.entity(arrow_entity).insert((
            SpriteBundle {
                texture: texture.unwrap(),
                transform: tr,
                ..Default::default()
            },
            NoPlacingHere,
        ));
    } else {
        cmd.entity(arrow_entity)
            .insert(TransformBundle::from_transform(tr));
    }
    cmd.entity(arrow_entity).with_children(|cmd| {
        if team == Team::Player {
            cmd.spawn((
                ArrowClickSensor,
                ClickSensor,
                Collider::capsule(vec2(-4., 0.0), vec2(3., 0.0), 12.),
                Sensor,
                TransformBundle::default(),
            ));
        }
        cmd.spawn((
            ArrowRobotSensor,
            Sensor,
            Collider::ball(96.),
            match team {
                Team::Player => {
                    coll_groups(ObjectGroup::PLAYER_ARROW_SENSOR, ObjectGroup::PLAYER_ROBOT)
                }
                Team::Enemy => {
                    coll_groups(ObjectGroup::ENEMY_ARROW_SENSOR, ObjectGroup::ENEMY_ROBOT)
                }
            },
            TransformBundle::default(),
            ActiveEvents::COLLISION_EVENTS,
        ));
    });
    arrow_entity
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
        (With<DragDropRequest>, With<DraggedArrow>),
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
                    inventory.arrow_count -= 1;
                    cmd.entity(entity).insert(DragDropConfirm);
                    let local_pos = drag_tr.translation - level_gtr.translation();
                    let arrow = spawn_arrow(
                        &mut cmd,
                        drag_tr.with_translation(local_pos),
                        dir.truncate(),
                        Team::Player,
                        Some(assets.arrow.clone()),
                        *grid_coords,
                    );
                    cmd.entity(level_entity).add_child(arrow);
                }
            }
        } else {
            cmd.entity(entity).remove::<DragDropRequest>();
        }
    }
}

fn check_click(
    mut cmd: Commands,
    mut ev_click_sensor: EventReader<ClickSensorEvent>,
    q_sensor: Query<&Parent, With<ArrowClickSensor>>,
    q_arrow: Query<(Entity, &GlobalTransform, &Team), With<Arrow>>,
    mut inventory: ResMut<Inventory>,
) {
    for ClickSensorEvent(sensor_entity) in ev_click_sensor.read() {
        if let Ok((arrow_entity, arrow_gtr, team)) = q_sensor
            .get(*sensor_entity)
            .map(|parent| parent.get())
            .and_then(|arrow_entity| q_arrow.get(arrow_entity))
        {
            if *team == Team::Player {
                inventory.arrow_count += 1;
                cmd.spawn((
                    Drag,
                    DragPos(arrow_gtr.translation().truncate()),
                    DraggedArrow,
                ));
                cmd.entity(arrow_entity).despawn_recursive();
            }
        }
    }
}

fn update_robot_motors(
    mut collision_events: EventReader<CollisionEvent>,
    q_robot_sensor: Query<&Parent, With<ArrowRobotSensor>>,
    q_arrow: Query<(&Arrow, &Team)>,
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
            if let (Ok((mut engine_dir, robot_team)), Ok((arrow, arrow_team))) = (
                q_robot.get_mut(other),
                q_robot_sensor
                    .get(sensor)
                    .and_then(|parent| q_arrow.get(parent.get())),
            ) {
                if robot_team == arrow_team {
                    engine_dir.0 = arrow.dir;
                }
            }
        }
    }
}
