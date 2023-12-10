use std::{f32::consts::PI, time::Duration};

use bevy::{
    math::{vec2, vec3},
    prelude::*,
};
use bevy_easings::{Ease, EaseFunction, EaseMethod, EasingType};

use crate::{game::GameState, load::TextureAssets};
use rand::prelude::*;
pub struct ExplosionPlugin;

impl Plugin for ExplosionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ExplosionEvent>()
            .add_systems(
                Update,
                (watch_for_explosion, run_explosion).run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                PostUpdate,
                particle_life.run_if(in_state(GameState::Playing)),
            );
    }
}

const EXPLOSION_DURATION: Duration = Duration::from_millis(300);
const EXPLOSION_PARTICLE_COUNT: u32 = 5;
const EXPLOSION_PARTICLE_LIFETIME: Duration = Duration::from_millis(300);
const EXPLOSION_PARTICLE_SPREAD: f32 = 8.;
const EXPLOSION_PARTICLE_RADIUS: f32 = 8.;

pub const EXPLOSION_COLORS: [Color; 3] = [
    Color::YELLOW,
    Color::rgba(1., 0., 0., 0.2),
    Color::rgba(0., 0., 0., 0.),
];

#[derive(Event)]
pub struct ExplosionEvent {
    pub location: Vec2,
    pub colors: [Color; 3],
    pub spread: f32,
    pub particle_radius: f32,
    pub particle_speed: Vec2,
    pub duration: Duration,
    pub particle_duration: Duration,
    pub particle_count: u32,
}

impl Default for ExplosionEvent {
    fn default() -> Self {
        Self {
            location: Default::default(),
            colors: EXPLOSION_COLORS,
            spread: EXPLOSION_PARTICLE_SPREAD,
            particle_radius: EXPLOSION_PARTICLE_RADIUS,
            particle_speed: Vec2::ZERO,
            duration: EXPLOSION_DURATION,
            particle_duration: EXPLOSION_PARTICLE_LIFETIME,
            particle_count: EXPLOSION_PARTICLE_COUNT,
        }
    }
}

#[derive(Component)]
struct ExplosionParticle {
    lifetime: Timer,
    speed: Vec2,
}

#[derive(Component)]
struct Explosion {
    lifetime: Timer,
    next_particle: Timer,
    colors: [Color; 3],
    spread: f32,
    particle_radius: f32,
    particle_speed: Vec2,
    particle_duration: Duration,
    particle_count: u32,
    //duration : Duration,
}

fn watch_for_explosion(mut cmd: Commands, mut ev_explosion: EventReader<ExplosionEvent>) {
    for ev in ev_explosion.read() {
        let particle_interval =
            Duration::from_secs_f32(ev.duration.as_secs_f32() / ev.particle_count as f32);
        let mut next_particle = Timer::new(particle_interval, TimerMode::Once);
        next_particle.tick(particle_interval);
        cmd.spawn((
            TransformBundle::from_transform(Transform::from_translation(ev.location.extend(3.0))),
            Explosion {
                lifetime: Timer::new(ev.duration, TimerMode::Once),
                next_particle,
                colors: ev.colors,
                spread: ev.spread,
                particle_radius: ev.particle_radius,
                particle_speed: ev.particle_speed,
                particle_duration: ev.particle_duration,
                particle_count: ev.particle_count,
            },
        ));
    }
}

fn run_explosion(
    mut cmd: Commands,
    mut q_explosion: Query<(Entity, &GlobalTransform, &mut Explosion)>,
    time: Res<Time>,
    textures: Res<TextureAssets>,
) {
    for (e_explosion, gtr, mut explosion) in &mut q_explosion {
        explosion.lifetime.tick(time.delta());
        explosion.next_particle.tick(time.delta());
        if explosion.next_particle.finished() {
            let mut rng = thread_rng();
            explosion.next_particle.reset();
            let delta_pos = Quat::from_rotation_z(rng.gen_range(0.0..2.0 * PI))
                .mul_vec3(vec3(1.0, 0.0, 0.0) * rng.gen_range(0.0..explosion.spread));
            spawn_particle(
                &mut cmd,
                gtr.translation() + delta_pos,
                &textures,
                &explosion.colors,
                explosion.particle_radius,
                explosion.particle_speed,
                explosion.particle_duration,
            );
        }
        if explosion.lifetime.finished() {
            cmd.entity(e_explosion).despawn_recursive();
        }
    }
}

fn spawn_particle(
    cmd: &mut Commands,
    pos: Vec3,
    textures: &Res<TextureAssets>,
    colors: &[Color; 3],
    radius: f32,
    speed: Vec2,
    duration: Duration,
) {
    let size = Some(Vec2::splat(radius * 2.));
    let ease_step_time = Duration::from_secs_f32(duration.as_secs_f32() / 2.0);
    cmd.spawn((
        ExplosionParticle {
            lifetime: Timer::new(duration, TimerMode::Once),
            speed,
        },
        SpriteBundle {
            transform: Transform::from_translation(pos),
            texture: textures.explosion_particle.clone(),
            sprite: Sprite {
                custom_size: Some(vec2(0.0, 0.0)),
                ..Default::default()
            },
            ..Default::default()
        },
        Sprite {
            custom_size: size,
            color: colors[0],
            ..Default::default()
        }
        .ease_to(
            Sprite {
                custom_size: size.map(|v| v / 3.0),
                color: colors[1],
                ..Default::default()
            },
            EaseMethod::Linear,
            EasingType::Once {
                duration: ease_step_time,
            },
        )
        .ease_to(
            Sprite {
                custom_size: Some(vec2(0.0, 0.0)),
                color: colors[2],
                ..Default::default()
            },
            EaseMethod::Linear,
            EasingType::Once {
                duration: ease_step_time,
            },
        ),
    ));
}

fn particle_life(
    mut cmd: Commands,
    mut q_particle: Query<(Entity, &mut ExplosionParticle, &mut Transform)>,
    time: Res<Time>,
) {
    for (e_particle, mut particle, mut particle_tr) in &mut q_particle {
        particle.lifetime.tick(time.delta());
        if particle.lifetime.finished() {
            cmd.entity(e_particle).despawn_recursive();
        } else {
            particle_tr.translation += particle.speed.extend(0.0);
        }
    }
}
