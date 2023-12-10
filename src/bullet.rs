use std::time::Duration;

use bevy::{
    math::{vec2, vec3},
    prelude::*,
};
use bevy_ecs_ldtk::LevelIid;
use bevy_rapier2d::prelude::*;

use crate::{
    explosion::ExplosionEvent,
    game::GameState,
    hp::Life,
    load::TextureAssets,
    physics::{coll_groups, ObjectGroup, Team},
    robot::{FireEvent, Robot},
};

pub struct BulletPlugin;

impl Plugin for BulletPlugin {
    fn build(&self, app: &mut App) {
        app //.add_systems(OnEnter(GameState::Playing), spawn_bullet)
            .add_systems(
                Update,
                (move_bullet, spawn_bullet, check_bullet_hit).run_if(in_state(GameState::Playing)),
            );
    }
}

const BULLET_LIFETIME: Duration = Duration::from_millis(2000);
const BULLET_SPEED: f32 = 4.;

#[derive(Component)]
pub struct Bullet {
    timer: Timer,
    direction: Vec2,
    angle: f32,
}

fn spawn_bullet(
    mut cmd: Commands,
    assets: Res<TextureAssets>,
    mut ev_fire: EventReader<FireEvent>,
    q_level: Query<Entity, With<LevelIid>>,
) {
    for ev in ev_fire.read() {
        if let Ok(e_level) = q_level.get_single() {
            let angle =
                vec2(1.0, 0.0).angle_between(ev.to_target.truncate() - ev.from_pos.truncate());
            let quat = Quat::from_rotation_z(angle);
            let e_bullet = cmd
                .spawn((
                    Bullet {
                        timer: Timer::new(BULLET_LIFETIME, TimerMode::Once),
                        direction: quat.mul_vec3(vec3(1.0, 0.0, 0.0)).truncate(),
                        angle,
                    },
                    SpriteBundle {
                        texture: assets.bullet.clone(),
                        transform: Transform::from_scale(Vec3::splat(0.5))
                            .with_rotation(quat)
                            .with_translation(vec3(ev.from_pos.x, ev.from_pos.y, 3.)),
                        ..Default::default()
                    },
                    // Collider::capsule_x(6.0, 2.0),
                    // Sensor,
                    ev.team,
                ))
                .id();
            cmd.entity(e_level).add_child(e_bullet);
        }
    }
}

fn move_bullet(
    mut cmd: Commands,
    mut q_bullet: Query<(Entity, &mut Bullet, &mut Transform)>,
    time: Res<Time>,
) {
    for (e_bullet, mut bullet, mut bullet_tr) in &mut q_bullet {
        bullet.timer.tick(time.delta());
        if bullet.timer.finished() {
            cmd.entity(e_bullet).despawn_recursive();
        } else {
            bullet_tr.translation += (bullet.direction * BULLET_SPEED).extend(0.0);
        }
    }
}

fn check_bullet_hit(
    mut cmd: Commands,
    rapier_context: Res<RapierContext>,
    q_bullet: Query<(Entity, &Bullet, &GlobalTransform, &Transform, &Team)>,
    mut q_other: Query<(&mut Life, &Team)>,
    mut ev_explosion: EventWriter<ExplosionEvent>,
) {
    let shape = Collider::capsule_x(6.0, 2.0);
    for (e_bullet, bullet, bullet_gtr, bullet_tr, bullet_team) in &q_bullet {
        let filter = QueryFilter {
            groups: Some(match *bullet_team {
                Team::Player => coll_groups(
                    ObjectGroup::PLAYER_BULLET,
                    ObjectGroup::ENEMY_ROBOT | ObjectGroup::ENEMY_PORTAL,
                ),
                Team::Enemy => coll_groups(
                    ObjectGroup::ENEMY_BULLET,
                    ObjectGroup::PLAYER_ROBOT | ObjectGroup::PLAYER_PORTAL,
                ),
            }),
            ..Default::default()
        };
        rapier_context.intersections_with_shape(
            bullet_gtr.translation().truncate(),
            bullet.angle,
            &shape,
            filter,
            |other| {
                let mut result = true;
                if let Ok((mut life, team)) = q_other.get_mut(other) {
                    if team != bullet_team {
                        cmd.entity(e_bullet).despawn_recursive();
                        life.curr_hp -= 1.0;
                        ev_explosion.send(ExplosionEvent {
                            location: bullet_tr.translation.truncate(),
                            particle_radius: 2.,
                            spread: 3.,
                            particle_speed: bullet.direction / 2.,
                            duration: Duration::from_millis(100),
                            ..Default::default()
                        });
                        result = false;
                        //println!("yoh");
                    }
                }
                result
            },
        );
    }
}
