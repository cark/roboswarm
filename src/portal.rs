use std::time::Duration;

use bevy::{math::vec2, prelude::*};
use bevy_ecs_ldtk::{prelude::*, utils::ldtk_grid_coords_to_translation};
use bevy_rapier2d::prelude::*;
use rand::Rng;

use crate::{
    explosion::ExplosionEvent,
    game::GameState,
    hp::{Dead, Life},
    load::TextureAssets,
    physics::{coll_groups, CollisionCache, ObjectGroup, Team},
    robot::{Robot, SpawnRobotEvent},
};

const PORTAL_SENSOR_WIDTH: f32 = 64.;
const PORTAL_STRENGTH: f32 = 100000.;
const PORTAL_START_HP: f32 = 50.;
const PORTAL_COLLIDER_RADIUS: f32 = 10.;
pub struct PortalPlugin;

impl Plugin for PortalPlugin {
    fn build(&self, app: &mut App) {
        //app.add_event::<PortalRobotSpawn>()
        app.add_systems(
            Update,
            (
                //                spawn_robot,
                check_sensor_collisions,
            )
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(Update, check_portal_robot_spawn)
        .add_systems(
            Update,
            check_added_portals.run_if(in_state(GameState::Playing)),
        )
        .add_systems(PostUpdate, check_dead.run_if(in_state(GameState::Playing)));
    }
}

#[derive(Bundle, LdtkEntity, Default)]
pub struct PlayerPortalBundle {
    #[ldtk_entity]
    portal: PortalBundle,
    #[grid_coords]
    grid_coords: GridCoords,
}

#[derive(Bundle, LdtkEntity, Default)]
pub struct EnemyPortalBundle {
    #[ldtk_entity]
    portal: PortalBundle,
    #[grid_coords]
    grid_coords: GridCoords,
}

#[derive(Bundle, Default)]
struct PortalBundle {
    portal: Portal,
    team: Team,
}

impl LdtkEntity for PortalBundle {
    fn bundle_entity(
        entity_instance: &EntityInstance,
        layer_instance: &LayerInstance,
        _tileset: Option<&Handle<Image>>,
        _tileset_definition: Option<&TilesetDefinition>,
        _asset_server: &AssetServer,
        _texture_atlases: &mut Assets<TextureAtlas>,
    ) -> Self {
        let spawn_rate = *entity_instance.get_float_field("spawn_rate").unwrap();
        let timer = if spawn_rate > 0. {
            let duration = Duration::from_secs_f32(1.0 / spawn_rate);
            let mut timer = Timer::new(duration, TimerMode::Once);
            timer.tick(duration);
            timer
        } else {
            Timer::new(Duration::MAX, TimerMode::Repeating)
        };
        let dir_point = *entity_instance.get_point_field("direction").unwrap();
        let size = IVec2::splat(layer_instance.grid_size);
        let dir = ldtk_grid_coords_to_translation(dir_point, layer_instance.c_hei, size);
        let pos = ldtk_grid_coords_to_translation(entity_instance.grid, layer_instance.c_hei, size);
        PortalBundle {
            portal: Portal {
                dir: dir - pos,
                spawn_timer: timer,
            },
            team: match entity_instance.identifier.as_str() {
                "PlayerPortal" => Team::Player,
                "EnemyPortal" => Team::Enemy,
                _ => unreachable!(),
            },
        }
    }
}

#[derive(Component, Default)]
pub struct Portal {
    pub spawn_timer: Timer,
    pub dir: Vec2,
}

#[derive(Component)]
struct PortalSensor;

fn check_portal_robot_spawn(
    mut ev_spawn_robot: EventWriter<SpawnRobotEvent>,
    mut ev_explosion: EventWriter<ExplosionEvent>,
    mut q_portal: Query<(&mut Portal, &Transform, &Team)>,
    time: Res<Time>,
) {
    for (mut portal, tr, team) in &mut q_portal {
        portal.spawn_timer.tick(time.delta());
        if portal.spawn_timer.finished() && tr.translation != Vec3::ZERO {
            portal.spawn_timer.reset();
            ev_spawn_robot.send(SpawnRobotEvent {
                dir: portal.dir.normalize(),
                pos: tr.translation.truncate(),
                team: *team,
                // team: Team::Enemy,
            });
            ev_explosion.send(ExplosionEvent {
                colors: [
                    Color::rgba(1.0, 1.0, 1.0, 0.8),
                    Color::rgba(0.9, 0.9, 0.9, 0.3),
                    Color::rgba(0.8, 0.8, 0.8, 0.0),
                ],
                location: tr.translation.truncate(),
                duration: Duration::from_secs_f32(0.3),
                particle_radius: 8.,
                spread: 12.,
                particle_duration: Duration::from_secs_f32(0.8),
                particle_count: 40,
                ..Default::default()
            })
        }
    }
}

pub fn check_added_portals(
    mut cmd: Commands,
    q_portal: Query<(Entity, &Portal, &Team, &Transform), Added<Portal>>,
    texture_assets: Res<TextureAssets>,
    //level_size: Res<LevelSize>,
) {
    for (portal_entity, portal, team, portal_tr) in &q_portal {
        cmd.entity(portal_entity)
            .insert((
                Life {
                    max_hp: PORTAL_START_HP,
                    curr_hp: PORTAL_START_HP,
                },
                SpriteBundle {
                    transform: *portal_tr,
                    texture: match team {
                        Team::Player => texture_assets.player_portal.clone(),
                        Team::Enemy => texture_assets.enemy_portal.clone(),
                    },
                    sprite: Sprite {
                        custom_size: Some(vec2(64.0, 64.0)),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                (
                    RigidBody::Fixed,
                    Collider::ball(PORTAL_COLLIDER_RADIUS),
                    match team {
                        Team::Player => coll_groups(
                            ObjectGroup::PLAYER_PORTAL,
                            ObjectGroup::ENEMY_ROBOT
                                | ObjectGroup::ENEMY_BULLET
                                | ObjectGroup::ENEMY_TARGETING_SENSOR,
                            //| ObjectGroup::ENE
                        ),
                        Team::Enemy => coll_groups(
                            ObjectGroup::ENEMY_PORTAL,
                            ObjectGroup::PLAYER_ROBOT
                                | ObjectGroup::PLAYER_BULLET
                                | ObjectGroup::PLAYER_TARGETING_SENSOR,
                        ),
                    },
                ),
            ))
            .with_children(|cmd| {
                cmd.spawn((
                    PortalSensor,
                    Sensor,
                    Collider::cuboid(portal.dir.length(), PORTAL_SENSOR_WIDTH / 2.),
                    TransformBundle::from_transform(
                        Transform::from_rotation(Quat::from_rotation_arc_2d(
                            vec2(1.0, 0.0),
                            portal.dir.normalize(),
                        ))
                        .with_translation(portal.dir.extend(0.0)),
                    ),
                    ActiveEvents::COLLISION_EVENTS,
                    match team {
                        Team::Player => coll_groups(
                            ObjectGroup::PLAYER_PORTAL_SENSOR,
                            ObjectGroup::PLAYER_ROBOT,
                        ),
                        Team::Enemy => {
                            coll_groups(ObjectGroup::ENEMY_PORTAL_SENSOR, ObjectGroup::ENEMY_ROBOT)
                        }
                    },
                ));
            });
    }
}

#[derive(Default)]
struct PortalSensorCache(CollisionCache);

fn check_sensor_collisions(
    mut collision_events: EventReader<CollisionEvent>,
    q_portal_sensor: Query<&Parent, With<PortalSensor>>,
    mut q_robot: Query<(&mut ExternalForce, &Team), With<Robot>>,
    mut sensor_cache: Local<PortalSensorCache>,
    q_portal: Query<(&Portal, &Team)>,
) {
    sensor_cache
        .0
        .cache_collisions(&mut collision_events, |entity| {
            q_portal_sensor.contains(entity)
        });
    let mut rng = rand::thread_rng();
    for (sensor_id, robot_set) in sensor_cache.0.cache.iter() {
        if let Ok(portal_id) = q_portal_sensor.get(*sensor_id).map(|parent| parent.get()) {
            for robot_id in robot_set.iter() {
                if let Ok((mut external_force, robot_team)) = q_robot.get_mut(*robot_id) {
                    if let Ok((portal, portal_team)) = q_portal.get(portal_id) {
                        if robot_team == portal_team {
                            let rx = (rng.gen::<f32>() - 0.5) / 2.;
                            let ry = (rng.gen::<f32>() - 0.5) / 2.;
                            external_force.force +=
                                (portal.dir.normalize() + vec2(rx, ry)) * PORTAL_STRENGTH;
                        }
                    }
                }
            }
        }
    }
}

fn check_dead(
    mut cmd: Commands,
    q_portal: Query<(Entity, &Transform), (With<Portal>, With<Dead>)>,
    mut ev_explosion: EventWriter<ExplosionEvent>,
) {
    for (e_portal, tr) in &q_portal {
        cmd.entity(e_portal).despawn_recursive();
        ev_explosion.send(ExplosionEvent {
            location: tr.translation.truncate(),
            ..Default::default()
        });
    }
}
