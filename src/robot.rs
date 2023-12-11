use std::time::Duration;

use bevy::{
    math::{vec2, vec3},
    prelude::*,
    sprite::Anchor,
};

use bevy_ecs_ldtk::LevelIid;
use bevy_rapier2d::prelude::*;

use crate::{
    explosion::ExplosionEvent,
    game::{GameState, LevelState},
    hp::{Dead, Life},
    levels::WallCollider,
    load::TextureAssets,
    physics::{coll_groups, ObjectGroup, Team},
};

use rand::prelude::*;

const CANON_ROTATION_SPEED: f32 = 3.0;
const TRAIN_ROTATION_SPEED: f32 = 5.0;
const ROBOT_COLLIDER_RADIUS: f32 = 7.0;
const ROBOT_COLLIDER_MASS: f32 = 1000.0;
const ROBOT_MOVEMENT_STRENGTH: f32 = 100000.0;
const ROBOT_DAMPING: f32 = 1.5;
const WHEEL_POSITIONS: [Vec2; 6] = [
    vec2(6.0, -6.0),
    vec2(-0.5, -7.0),
    vec2(-6.0, -6.0),
    vec2(6.0, 6.0),
    vec2(-0.5, 7.0),
    vec2(-6.0, 6.0),
];
const CANON_COOLDOWN: Duration = Duration::from_secs(1);
const CANON_VARIANCE: Duration = Duration::from_millis(900);
const ROBOT_START_HP: f32 = 5.0;

//const ROBOT_STEERING_SENSOR_RADIUS: f32 = 32.;

pub struct RobotPlugin;

impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        app //.add_systems(OnEnter(GameState::Playing), spawn_robot)
            .add_event::<SpawnRobotEvent>()
            .add_event::<FireEvent>()
            .add_systems(
                PreUpdate,
                (reset_robot_strength).run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (
                    check_for_target,
                    set_canon_target,
                    rotate_canon.after(set_canon_target),
                    fire_canon,
                    flash_nuzzle,
                    // set_engine_dir,
                    // apply_engine_dir.after(set_engine_dir),
                    apply_engine_dir,
                    (
                        (
                            check_spawn_robot,
                            steering_forces,
                            rotate_wheel_train,
                            rotate_wheel,
                        ),
                        set_last_pos,
                    )
                        .chain(),
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                PostUpdate,
                (last_strength_check, check_dead)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Component)]
pub struct Robot;

#[derive(Component)]
pub struct Turret;

#[derive(Component)]
pub struct Canon;

#[derive(Component, Default)]
struct CanonTarget(Vec2);

#[derive(Component, Default)]
pub struct EngineDir(pub Vec2);

#[derive(Component)]
struct RobotBody;

#[derive(Component)]
pub struct Leg;

#[derive(Component)]
pub struct WheelTrain;

#[derive(Component, Default)]
pub struct LastPos(Vec2);

#[derive(Component)]
pub struct Wheel;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct RobotTarget(Entity);

#[derive(Component, Default)]
pub struct CanonCooldown(Timer);

#[derive(Component)]
pub struct NuzzleFlash(Timer);

#[derive(Event)]
pub struct FireEvent {
    pub from_pos: Vec3,
    pub to_target: Vec3,
    pub team: Team,
}

// #[derive(Component)]
// pub struct RobotSteeringSensor;

#[derive(Event)]
pub struct SpawnRobotEvent {
    pub pos: Vec2,
    pub dir: Vec2,
    pub team: Team,
}

fn check_spawn_robot(
    mut cmd: Commands,
    mut ev_spawn_robot: EventReader<SpawnRobotEvent>,
    assets: Res<TextureAssets>,
    q_level: Query<Entity, With<LevelIid>>,
) {
    if let Ok(e_level) = q_level.get_single() {
        for ev in ev_spawn_robot.read() {
            //info!("spawn robot event read {:?}", ev.pos);
            let pos = ev.pos.extend(4.0);
            // info!("team: {}", ev.team as u32);
            let robot_id = cmd
                .spawn((
                    ev.team,
                    Robot,
                    Life {
                        max_hp: ROBOT_START_HP,
                        curr_hp: ROBOT_START_HP,
                    },
                    EngineDir::default(),
                    (
                        RigidBody::Dynamic,
                        Collider::ball(ROBOT_COLLIDER_RADIUS),
                        AdditionalMassProperties::Mass(ROBOT_COLLIDER_MASS),
                        ExternalForce {
                            force: vec2(0.0, 0.0),
                            torque: 0.0,
                        },
                        ExternalImpulse::default(),
                        Damping {
                            linear_damping: ROBOT_DAMPING,
                            angular_damping: 0.0,
                        },
                        LockedAxes::ROTATION_LOCKED,
                        Friction {
                            coefficient: 0.0,
                            ..Default::default()
                        },
                        Restitution {
                            coefficient: 0.5,
                            ..Default::default()
                        },
                        match ev.team {
                            Team::Player => coll_groups(
                                ObjectGroup::PLAYER_ROBOT,
                                ObjectGroup::ENEMY_ROBOT
                                    | ObjectGroup::WALL
                                    | ObjectGroup::PLAYER_PORTAL_SENSOR
                                    | ObjectGroup::PLAYER_ROBOT
                                    | ObjectGroup::PLAYER_ARROW_SENSOR
                                    | ObjectGroup::ROBOT_STEERING_SENSOR
                                    | ObjectGroup::ENEMY_TARGETING_SENSOR
                                    | ObjectGroup::ENEMY_BULLET
                                    | ObjectGroup::ENEMY_PORTAL
                                    | ObjectGroup::PLAYER_FORK_SENSOR
                                    | ObjectGroup::PLAYER_GROUPER_SENSOR
                                    | ObjectGroup::PLAYER_DEFENDER_SENSOR,
                            ),
                            Team::Enemy => coll_groups(
                                ObjectGroup::ENEMY_ROBOT,
                                ObjectGroup::PLAYER_ROBOT
                                    | ObjectGroup::WALL
                                    | ObjectGroup::ENEMY_PORTAL_SENSOR
                                    | ObjectGroup::ENEMY_ROBOT
                                    | ObjectGroup::ENEMY_ARROW_SENSOR
                                    | ObjectGroup::ROBOT_STEERING_SENSOR
                                    | ObjectGroup::PLAYER_TARGETING_SENSOR
                                    | ObjectGroup::PLAYER_BULLET
                                    | ObjectGroup::PLAYER_PORTAL
                                    | ObjectGroup::ENEMY_FORK_SENSOR
                                    | ObjectGroup::ENEMY_GROUPER_SENSOR
                                    | ObjectGroup::ENEMY_DEFENDER_SENSOR,
                            ),
                        },
                    ),
                    TransformBundle::from_transform(Transform::from_translation(pos)),
                    VisibilityBundle::default(),
                ))
                .with_children(|cmd| {
                    cmd.spawn((
                        RobotBody,
                        SpriteBundle {
                            texture: assets.robot_body.clone(),
                            transform: Transform::from_translation(vec3(0., 0., 1.)),
                            sprite: Sprite {
                                color: ev.team.tint(),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    ));
                    cmd.spawn((
                        Turret,
                        SpriteBundle {
                            texture: assets.robot_turret.clone(),
                            transform: Transform::from_translation(vec3(0.0, 0.0, 1.2)),
                            ..Default::default()
                        },
                    ));
                    cmd.spawn((
                        Canon,
                        CanonCooldown({
                            let mut t = Timer::new(CANON_COOLDOWN, TimerMode::Repeating);
                            t.tick(CANON_COOLDOWN);
                            t
                        }),
                        SpriteBundle {
                            texture: assets.robot_canon.clone(),
                            transform: Transform::from_translation(vec3(0.0, 0.0, 1.1)),
                            ..Default::default()
                        },
                        CanonTarget::default(),
                        ev.team,
                    ))
                    .with_children(|cmd| {
                        cmd.spawn((
                            NuzzleFlash({
                                let mut result =
                                    Timer::new(Duration::from_millis(100), TimerMode::Once);
                                result.set_elapsed(Duration::from_millis(100));
                                result
                            }),
                            SpriteBundle {
                                texture: assets.nuzzle_flash.clone(),
                                transform: Transform::from_xyz(8., 0., 2.)
                                    .with_scale(Vec3::splat(0.5)),
                                visibility: Visibility::Hidden,
                                ..Default::default()
                            },
                        ));
                    });
                    cmd.spawn((
                        LastPos::default(),
                        SpriteBundle {
                            texture: assets.robot_train.clone(),
                            transform: Transform::from_translation(vec3(0.0, 0.0, 0.9)),
                            ..Default::default()
                        },
                        WheelTrain,
                    ))
                    .with_children(|cmd| {
                        for wp in WHEEL_POSITIONS {
                            cmd.spawn((
                                Wheel,
                                SpriteBundle {
                                    transform: Transform::from_translation(wp.extend(0.8)),
                                    sprite: Sprite {
                                        anchor: Anchor::Center,
                                        custom_size: Some(vec2(4.0, 2.0)),
                                        color: Color::BLACK,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                                LastPos(wp),
                                //LastPos::default(),
                            ));
                        }
                    });
                })
                .id();
            cmd.entity(e_level).add_child(robot_id);
        }
    }
}

fn set_canon_target(
    mut cmd: Commands,
    mut canon_q: Query<(&GlobalTransform, &mut CanonTarget, &Parent), With<Canon>>,
    robot_q: Query<(&EngineDir, Option<&RobotTarget>, &GlobalTransform), With<Robot>>,
    other_q: Query<&GlobalTransform>,
) {
    for (cannon_gtr, mut canon_target_dir, parent) in canon_q.iter_mut() {
        if let Ok((engine_dir, robot_target, _)) = robot_q.get(parent.get()) {
            canon_target_dir.0 = engine_dir.0;
            if let Some(RobotTarget(e_other)) = robot_target {
                if let Ok(other_gtr) = other_q.get(*e_other) {
                    let dir_vec =
                        other_gtr.translation().truncate() - cannon_gtr.translation().truncate();
                    if dir_vec.length() > 200. {
                        cmd.entity(parent.get()).remove::<RobotTarget>();
                    } else {
                        canon_target_dir.0 = dir_vec.normalize();
                    }
                }
            }
        }
    }
}

fn fire_canon(
    //mut cmd: Commands,
    mut canon_q: Query<
        (
            &GlobalTransform,
            &Parent,
            &mut CanonCooldown,
            &Children,
            &Team,
        ),
        With<Canon>,
    >,
    robot_q: Query<Option<&RobotTarget>, With<Robot>>,
    other_q: Query<&GlobalTransform, &Transform>,
    mut nuzzle_q: Query<(&mut NuzzleFlash, &GlobalTransform)>,
    time: Res<Time>,
    mut ev_fire: EventWriter<FireEvent>,
    q_level: Query<&GlobalTransform, With<LevelIid>>,
    level_state: Res<State<LevelState>>,
) {
    if *level_state != LevelState::Playing {
        return;
    }
    for (canon_gtr, parent, mut cooldown, children, team) in &mut canon_q {
        cooldown.0.tick(time.delta());
        if cooldown.0.finished() {
            let variance = CANON_VARIANCE.as_secs_f32();
            let cd = CANON_COOLDOWN.as_secs_f32();
            let ms = variance * rand::thread_rng().gen::<f32>() - variance / 2.;
            cooldown.0.set_duration(Duration::from_secs_f32(cd + ms));
            cooldown.0.reset();
            if let Ok(Some(RobotTarget(e_other))) = robot_q.get(parent.get()) {
                if let Ok(other_gtr) = other_q.get(*e_other) {
                    let firing_to = (other_gtr.translation() - canon_gtr.translation()).normalize();
                    let curr_dir = (canon_gtr.transform_point(vec3(1.0, 0.0, 0.0))
                        - canon_gtr.translation())
                    .normalize();
                    let quat =
                        Quat::from_rotation_arc_2d(curr_dir.truncate(), firing_to.truncate());
                    let (_axis, angle) = quat.to_axis_angle();
                    if angle.abs() < 0.1 {
                        for child in children.iter() {
                            if let Ok((mut nuzzle, nuzzle_gtr)) = nuzzle_q.get_mut(*child) {
                                if let Ok(level_gtr) = q_level.get_single() {
                                    nuzzle.0.reset();
                                    ev_fire.send(FireEvent {
                                        from_pos: nuzzle_gtr.translation()
                                            - level_gtr.translation(),
                                        to_target: other_gtr.translation()
                                            - level_gtr.translation(),
                                        team: *team,
                                    });
                                }
                            }
                        }
                        cooldown.0.reset();
                    }
                }
            }
        }
    }
}

fn flash_nuzzle(mut q_nuzzle: Query<(&mut NuzzleFlash, &mut Visibility)>, time: Res<Time>) {
    for (mut nuzzle, mut visibility) in &mut q_nuzzle {
        nuzzle.0.tick(time.delta());
        if nuzzle.0.finished() {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Visible;
        }
    }
}

fn rotate_canon(
    mut q_canon: Query<(&GlobalTransform, &mut Transform, &CanonTarget), With<Canon>>,
    time: Res<Time>,
) {
    for (global_tr, mut tr, target_dir) in q_canon.iter_mut() {
        // cannon rotation
        const RIGHT: Vec3 = vec3(1.0, 0.0, 0.0);
        let curr_vec = global_tr.transform_point(RIGHT) - global_tr.translation();
        let max_rotation = CANON_ROTATION_SPEED * time.delta_seconds();
        let mut quat = Quat::from_rotation_arc_2d(curr_vec.truncate(), target_dir.0);
        let (axis, angle) = quat.to_axis_angle();
        if angle.abs() <= max_rotation {
            tr.rotate(quat);
        } else {
            quat = Quat::from_axis_angle(axis, max_rotation * angle.signum());
            tr.rotate(quat);
        }
    }
}

fn reset_robot_strength(mut q_robot: Query<&mut ExternalForce, With<Robot>>) {
    for mut external_force in q_robot.iter_mut() {
        external_force.force = vec2(0.0, 0.0);
    }
}

fn apply_engine_dir(
    mut q_robot: Query<(&EngineDir, &mut ExternalForce, Option<&RobotTarget>), With<Robot>>,
    level_state: Res<State<LevelState>>,
) {
    if *level_state == LevelState::Playing {
        for (engine_dir, mut external_force, robot_target) in q_robot.iter_mut() {
            if robot_target.is_none() {
                external_force.force += engine_dir.0 * ROBOT_MOVEMENT_STRENGTH;
            }
        }
    }
}

fn set_last_pos(mut q_entity: Query<(&mut LastPos, &GlobalTransform)>) {
    for (mut last_pos, global_tr) in q_entity.iter_mut() {
        last_pos.0 = global_tr.translation().truncate();
    }
}

fn rotate_wheel_train(
    mut q_wheel_train: Query<(&LastPos, &mut Transform, &GlobalTransform), With<WheelTrain>>,
    time: Res<Time>,
) {
    for (last_pos, mut tr, global_tr) in q_wheel_train.iter_mut() {
        let dir = global_tr.translation().truncate() - last_pos.0;
        let speed = dir.length() / time.delta_seconds();
        if speed > 3.0 {
            let ndir = dir.normalize();
            let old_dir = (global_tr.transform_point(vec3(1.0, 0.0, 0.0))
                - global_tr.translation())
            .normalize()
            .truncate();
            let mut angle = old_dir.angle_between(ndir);
            let max_angle = TRAIN_ROTATION_SPEED * time.delta_seconds();
            // info!("{} {}", angle, max_angle);
            if angle.abs() > max_angle {
                angle = max_angle * angle.signum();
            }
            tr.rotate(Quat::from_rotation_z(angle));
            // tr.rotation = Quat::from_rotation_arc_2d(vec2(1.0, 0.0), dir.normalize());
        } else {
            //tr.rotation = Quat::IDENTITY;
        }
    }
}

fn rotate_wheel(
    mut q_wheel: Query<(&LastPos, &mut Transform, &GlobalTransform), With<Wheel>>,
    time: Res<Time>,
) {
    for (last_pos, mut tr, global_tr) in q_wheel.iter_mut() {
        let dir = global_tr.translation().truncate() - last_pos.0;
        let speed = dir.length() / time.delta_seconds();
        if speed > 3.0 {
            let ndir = dir.normalize();
            let old_dir = (global_tr.transform_point(vec3(1.0, 0.0, 0.0))
                - global_tr.translation())
            .normalize()
            .truncate();
            let angle = old_dir.angle_between(ndir);
            tr.rotate(Quat::from_rotation_z(angle));
        } else {
            //tr.rotation = Quat::IDENTITY;
        }
    }
}

fn steering_forces(
    rapier_context: Res<RapierContext>,
    mut q_robot_force: Query<(Entity, &mut ExternalForce), With<Robot>>,
    q_robot: Query<&GlobalTransform, With<Robot>>,
    q_wall: Query<&GlobalTransform, With<WallCollider>>,
    // q_other: Query<&GlobalTransform, With<Collider>>,
) {
    let radius = 32.;
    let filter = QueryFilter {
        groups: Some(coll_groups(
            ObjectGroup::ROBOT_STEERING_SENSOR,
            ObjectGroup::PLAYER_ROBOT | ObjectGroup::ENEMY_ROBOT | ObjectGroup::WALL,
        )),
        ..Default::default()
    };
    let shape = Collider::ball(radius);
    for (e_robot, mut ext_force) in &mut q_robot_force {
        let gtr = q_robot.get(e_robot).unwrap();
        let shape_pos = gtr.translation().truncate();
        rapier_context.intersections_with_shape(shape_pos, 0.0f32, &shape, filter, |entity| {
            if entity != e_robot {
                if let Ok(wall_gtr) = q_wall.get(entity) {
                    let vec = gtr.translation().truncate() - wall_gtr.translation().truncate();
                    let len = vec.length();
                    let strength = (1.5 * radius - len).abs().powf(1.1) * 1750.;
                    let unit_vec = vec.normalize_or_zero();
                    ext_force.force += unit_vec * strength;
                } else if let Ok(other_gtr) = q_robot.get(entity) {
                    let vec = gtr.translation().truncate() - other_gtr.translation().truncate();
                    let len = vec.length();
                    let strength = (1.0 * radius - len).abs().powf(1.3) * 3000.;
                    let unit_vec = vec.normalize_or_zero();
                    ext_force.force += unit_vec * strength;
                }
            }
            true
        })
    }
}

fn last_strength_check(mut q_robot: Query<&mut ExternalForce, With<Robot>>) {
    for mut ext_force in &mut q_robot {
        if ext_force.force.length() < 25000. {
            //info("yoh");
            ext_force.force = Vec2::ZERO;
        }
    }
}

fn check_for_target(
    mut cmd: Commands,
    rapier_context: Res<RapierContext>,
    q_robot: Query<(Entity, &GlobalTransform, &Team), With<Robot>>,
    q_other: Query<(Entity, &GlobalTransform, &Team)>,
) {
    let radius = 96.;
    let shape = Collider::ball(radius);
    for (e_robot, robot_gtr, robot_team) in &q_robot {
        let filter = QueryFilter {
            groups: Some(match *robot_team {
                Team::Player => coll_groups(
                    ObjectGroup::PLAYER_TARGETING_SENSOR,
                    ObjectGroup::ENEMY_ROBOT | ObjectGroup::ENEMY_PORTAL,
                ),
                Team::Enemy => coll_groups(
                    ObjectGroup::ENEMY_TARGETING_SENSOR,
                    ObjectGroup::PLAYER_ROBOT | ObjectGroup::PLAYER_PORTAL,
                ),
            }),
            ..Default::default()
        };
        let mut min: Option<(Entity, f32)> = None;
        rapier_context.intersections_with_shape(
            robot_gtr.translation().truncate(),
            0.0,
            &shape,
            filter,
            |e_other| {
                if let Ok((_, other_gtr, _)) = q_other.get(e_other) {
                    let dist = other_gtr.translation().distance(robot_gtr.translation());
                    if min.is_none() || min.unwrap().1 > dist {
                        min = Some((e_other, dist));
                    }
                }
                true
            },
        );
        cmd.entity(e_robot).remove::<RobotTarget>();
        if let Some((e_other, _)) = min {
            cmd.entity(e_robot).try_insert(RobotTarget(e_other));
        }
    }
}

fn check_dead(
    mut cmd: Commands,
    q_robot: Query<(Entity, &Transform), (With<Robot>, With<Dead>)>,
    mut ev_explosion: EventWriter<ExplosionEvent>,
) {
    for (e_robot, tr) in &q_robot {
        cmd.entity(e_robot).despawn_recursive();
        ev_explosion.send(ExplosionEvent {
            location: tr.translation.truncate(),
            ..Default::default()
        });
    }
}
