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
    plugin::RapierContext,
};

use crate::{
    game::GameState,
    game_camera::MouseWorldCoords,
    inventory::Inventory,
    levels::{LevelLoadedEvent, LevelSize, WallCache},
    load::TextureAssets,
    mouse::{
        ClickSensor, ClickSensorEvent, Drag, DragCancelConfirm, DragCancelRequest, DragDropConfirm,
        DragDropRequest, DragPos,
    },
    physics::{coll_groups, ObjectGroup, Team},
    portal::Portal,
    robot::{EngineDir, Robot},
};

pub struct ArrowPlugin;

impl Plugin for ArrowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                spawn_draggable_arrow,
                validate_drag,
                check_click,
                (drag_cancel_request, drop_request).chain(),
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
}

fn extract_ldtk_dir(entity_instance: &EntityInstance) -> LdtkDir {
    LdtkDir(*entity_instance.get_point_field("direction").unwrap())
}

#[derive(Component)]
pub struct DraggedArrow;

#[derive(Component, PartialEq)]
enum DragState {
    Dragging,
    SettingDirection(Transform),
}

#[derive(Component)]
pub struct Arrow {
    dir: Vec2,
}

#[derive(Component)]
struct ValidDrag;

#[derive(Component)]
struct ArrowClickSensor;

#[derive(Component)]
struct ArrowRobotSensor;

fn spawn_draggable_arrow(
    mut cmd: Commands,
    assets: Res<TextureAssets>,
    q_drag: Query<(Entity, &DragPos), (With<DraggedArrow>, Added<Drag>)>,
) {
    for (entity, drag_pos) in &q_drag {
        cmd.entity(entity).insert((
            SpriteBundle {
                transform: Transform::from_translation(drag_pos.0.extend(0.0)),
                texture: assets.arrow.clone(),
                ..Default::default()
            },
            DragState::Dragging,
        ));
    }
}

fn validate_drag(
    mut cmd: Commands,
    mut q_drag: Query<(Entity, &mut Transform, &mut Sprite, &DragState), With<DraggedArrow>>,
    mouse_pos: Res<MouseWorldCoords>,
    q_level: Query<(&GlobalTransform, &WallCache), With<LevelIid>>,
    level_size: Res<LevelSize>,
    q_portal: Query<&GridCoords, With<Portal>>, //wall_cache: Res<WallCache>,
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
                            && q_portal.iter().all(|grid_coord| {
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
                            cmd.entity(entity).insert(ValidDrag);
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

fn drag_cancel_request(mut cmd: Commands, q_drag: Query<Entity, Added<DragCancelRequest>>) {
    for entity in &q_drag {
        cmd.entity(entity).insert(DragCancelConfirm);
    }
}

fn fixup_enemy_arrow(
    mut cmd: Commands,
    q_arrow: Query<(Entity, &LdtkDir, &GridCoords, &Transform)>,
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
) -> Entity {
    let arrow_entity = cmd.spawn((Arrow { dir }, team)).id();
    if team == Team::Player {
        cmd.entity(arrow_entity).insert(SpriteBundle {
            texture: texture.unwrap(),
            transform: tr,
            ..Default::default()
        });
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
            Collider::ball(if team == Team::Player { 96. } else { 96. * 2. }),
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
        (Entity, &Transform, Option<&ValidDrag>, &mut DragState),
        With<DragDropRequest>,
    >,
    assets: Res<TextureAssets>,
    mut inventory: ResMut<Inventory>,
    q_level: Query<(Entity, &GlobalTransform), With<LevelIid>>,
) {
    for (entity, drag_tr, maybe_valid, mut drag_state) in &mut q_drag {
        if maybe_valid.is_some() {
            match *drag_state {
                DragState::Dragging => {
                    //println!("to setting direction {}", drag_tr.translation);
                    *drag_state = DragState::SettingDirection(*drag_tr);
                    cmd.entity(entity).remove::<DragDropRequest>();
                }
                DragState::SettingDirection(_) => {
                    let dir = drag_tr.rotation.mul_vec3(vec3(1.0, 0.0, 0.0));
                    let (level_entity, level_gtr) = q_level.single();
                    //println!("setting_direction {dir}");
                    inventory.arrow_count -= 1;
                    cmd.entity(entity).insert(DragDropConfirm);
                    let arrow = spawn_arrow(
                        &mut cmd,
                        drag_tr.with_translation(drag_tr.translation - level_gtr.translation()),
                        dir.truncate(),
                        Team::Player,
                        Some(assets.arrow.clone()),
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
    q_arrow: Query<(Entity, &GlobalTransform, &Team)>,
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
