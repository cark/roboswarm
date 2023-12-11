use std::{collections::VecDeque, f32::consts::PI};

use crate::{
    draggable::{drag_cancel_request, draggable_spawner, validate_drag, DragState, ValidDrag},
    game::GameState,
    inventory::Inventory,
    levels::NoPlacingHere,
    load::TextureAssets,
    mouse::{ClickSensor, ClickSensorEvent, Drag, DragDropConfirm, DragDropRequest, DragPos},
    physics::{coll_groups, ObjectGroup, Team},
    robot::{EngineDir, Robot},
};
use bevy::{
    math::{vec2, vec3},
    prelude::*,
};
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

pub struct DefenderPlugin;

impl Plugin for DefenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (
                    draggable_spawner::<DraggedDefender>("defender.png"),
                    validate_drag::<DraggedDefender>,
                    drag_cancel_request::<DraggedDefender>,
                    drop_request,
                    check_click,
                ),
                update_robot_motors,
                // fixup_enemy_grouper,
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

#[derive(Component)]
pub struct DraggedDefender;

#[derive(Component)]
struct DefenderClickSensor;

#[derive(Component)]
struct DefenderRobotSensor;

#[derive(Component)]
pub struct Defender {
    dir: Vec2,
    group: VecDeque<Entity>,
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
        (With<DragDropRequest>, With<DraggedDefender>),
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
                    inventory.defender_count -= 1;
                    cmd.entity(entity).insert(DragDropConfirm);
                    let local_pos = drag_tr.translation - level_gtr.translation();
                    //info(grid_coords);
                    let defender = spawn_defender(
                        &mut cmd,
                        drag_tr.with_translation(local_pos),
                        dir.truncate(),
                        Team::Player,
                        Some(assets.defender.clone()),
                        *grid_coords,
                    );
                    cmd.entity(level_entity).add_child(defender);
                }
            }
        } else {
            cmd.entity(entity).remove::<DragDropRequest>();
        }
    }
}

fn spawn_defender(
    cmd: &mut Commands,
    tr: Transform,
    dir: Vec2,
    team: Team,
    texture: Option<Handle<Image>>,
    grid_coords: GridCoords,
) -> Entity {
    let spawned_entity = cmd
        .spawn((
            Defender {
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
                DefenderClickSensor,
                ClickSensor,
                Collider::capsule(vec2(-4., 0.0), vec2(3., 0.0), 12.),
                Sensor,
                TransformBundle::default(),
            ));
        }
        cmd.spawn((
            DefenderRobotSensor,
            Sensor,
            Collider::ball(96.),
            match team {
                Team::Player => coll_groups(
                    ObjectGroup::PLAYER_DEFENDER_SENSOR,
                    ObjectGroup::PLAYER_ROBOT,
                ),
                Team::Enemy => {
                    coll_groups(ObjectGroup::ENEMY_DEFENDER_SENSOR, ObjectGroup::ENEMY_ROBOT)
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
    q_sensor: Query<&Parent, With<DefenderClickSensor>>,
    q_defender: Query<(Entity, &GlobalTransform, &Team), With<Defender>>,
    mut inventory: ResMut<Inventory>,
) {
    for ClickSensorEvent(sensor_entity) in ev_click_sensor.read() {
        if let Ok((defender_entity, defender_gtr, team)) = q_sensor
            .get(*sensor_entity)
            .map(|parent| parent.get())
            .and_then(|defender_entity| q_defender.get(defender_entity))
        {
            if *team == Team::Player {
                inventory.defender_count += 1;
                cmd.spawn((
                    Drag,
                    DragPos(defender_gtr.translation().truncate()),
                    DraggedDefender,
                ));
                cmd.entity(defender_entity).despawn_recursive();
            }
        }
    }
}

const GROUP_SIZE: usize = 25;

#[derive(Resource, Default)]
struct DeleteRobots(Vec<Entity>);

fn update_robot_motors(
    mut collision_events: EventReader<CollisionEvent>,
    q_robot_sensor: Query<&Parent, With<DefenderRobotSensor>>,
    mut q_defender: Query<(&mut Defender, &Team, &Transform)>,
    mut q_robot: Query<(&mut EngineDir, &Team, &Transform), With<Robot>>,
    mut delete_robot: Local<DeleteRobots>,
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
                if let (Ok((_, robot_team, _)), Ok((mut defender, defender_team, _))) = (
                    q_robot.get(other),
                    q_robot_sensor
                        .get(sensor)
                        .and_then(|parent| q_defender.get_mut(parent.get())),
                ) {
                    if robot_team == defender_team {
                        defender.group.push_back(other);
                        while defender.group.len() >= GROUP_SIZE {
                            if let Ok((mut engine_dir, _, _)) =
                                q_robot.get_mut(defender.group.pop_front().unwrap())
                            {
                                engine_dir.0 = defender.dir;
                            }
                        }
                    }
                }
            }
            CollisionEvent::Stopped(_e1, _e2, _ev_flags) => {} // CollisionEvent::Stopped(e1, e2, ev_flags) => {
                                                               //     let (sensor, other) =
                                                               //         match (q_robot_sensor.contains(*e1), q_robot_sensor.contains(*e2)) {
                                                               //             (true, false) => (*e1, *e2),
                                                               //             (false, true) => (*e2, *e1),
                                                               //             _ => continue,
                                                               //         };
                                                               //     if ev_flags.intersects(CollisionEventFlags::REMOVED) {
                                                               //         if let Ok((mut defender, _, _)) = q_robot_sensor
                                                               //             .get(sensor)
                                                               //             .and_then(|parent| q_defender.get_mut(parent.get()))
                                                               //         {
                                                               //             if let Some(index) = defender.group.iter().position(|&item| item == other) {
                                                               //                 println!("removed 1");
                                                               //                 defender.group.remove(index);
                                                               //             }
                                                               //         }
                                                               //     }
                                                               // }
        }
    }
    let remove_robots = &mut delete_robot.0;
    for (mut defender, _, d_tr) in &mut q_defender {
        remove_robots.clear();
        for e_robot in defender.group.iter() {
            if let Ok((mut engine_dir, _, r_tr)) = q_robot.get_mut(*e_robot) {
                let to_defender = (d_tr.translation - r_tr.translation).normalize();
                let rotated = Quat::from_rotation_z(-PI / 4.0)
                    .mul_vec3(to_defender)
                    .truncate();
                engine_dir.0 = rotated * 1.5; //to_grouper.truncate();
            } else {
                remove_robots.push(*e_robot);
            }
        }
        for e_robot in remove_robots.iter() {
            if let Some(index) = defender.group.iter().position(|item| item == e_robot) {
                defender.group.remove(index);
            }
        }
    }
}
