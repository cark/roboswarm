use bevy::prelude::*;
use bevy_easings::EasingsPlugin;
use bevy_ecs_ldtk::LdtkPlugin;

use crate::arrow::ArrowPlugin;
use crate::bullet::BulletPlugin;
use crate::explosion::ExplosionPlugin;
use crate::game_ui::GameUiPlugin;
use crate::hp::HpPlugin;
use crate::inventory::InventoryPlugin;
use crate::levels::LevelsPlugin;
use crate::load::LoadPlugin;
use crate::menu::MenuPlugin;
use crate::mouse::MousePlugin;
use crate::portal::PortalPlugin;
use crate::{game_camera::GameCameraPlugin, robot::RobotPlugin};

use bevy_rapier2d::plugin::{NoUserData, RapierPhysicsPlugin};
use bevy_rapier2d::prelude::*;

const PIXELS_PER_METER: f32 = 8.0;

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum GameState {
    #[default]
    Loading,
    LoadingLevels,
    Menu,
    Playing,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            (
                LoadPlugin,
                RobotPlugin,
                GameCameraPlugin,
                RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(PIXELS_PER_METER),
                LdtkPlugin,
                MousePlugin,
                LevelsPlugin,
                MenuPlugin,
                PortalPlugin,
                InventoryPlugin,
                GameUiPlugin,
            ),
            ArrowPlugin,
            BulletPlugin,
            HpPlugin,
            ExplosionPlugin,
            EasingsPlugin,
        ))
        .insert_resource(RapierConfiguration {
            gravity: Vect::new(0.0, 0.0),
            ..Default::default()
        })
        .add_state::<GameState>();
        app.add_plugins(RapierDebugRenderPlugin::default());
    }
}
