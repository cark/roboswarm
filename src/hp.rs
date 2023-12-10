use bevy::prelude::*;

use crate::game::GameState;

pub struct HpPlugin;

impl Plugin for HpPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, watch_life.run_if(in_state(GameState::Playing)));
    }
}

#[derive(Component, Default)]
pub struct Life {
    pub max_hp: f32,
    pub curr_hp: f32,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Dead;

fn watch_life(mut cmd: Commands, q_life: Query<(Entity, &Life), Changed<Life>>) {
    for (entity, life) in &q_life {
        if life.curr_hp <= 0. {
            cmd.entity(entity).try_insert(Dead);
            //cmd.entity(entity).despawn_recursive();
        }
    }
}
