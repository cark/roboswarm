use bevy::prelude::*;
use bevy::utils::info;
use bevy_easings::EasingsPlugin;
use bevy_ecs_ldtk::assets::LdtkProject;
use bevy_ecs_ldtk::LdtkPlugin;

use crate::arrow::ArrowPlugin;
use crate::bullet::BulletPlugin;
use crate::explosion::ExplosionPlugin;
use crate::fork::ForkPlugin;
use crate::game_ui::{GameUiPlugin, MainMenuEvent};
use crate::grouper::GrouperPlugin;
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

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum LevelState {
    #[default]
    WaitingLevelSpawn,
    Playing,
    Win,
    Loss,
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
                InventoryPlugin,
                GameUiPlugin,
            ),
            (PortalPlugin, ArrowPlugin, ForkPlugin, GrouperPlugin),
            BulletPlugin,
            HpPlugin,
            ExplosionPlugin,
            EasingsPlugin,
        ))
        .insert_resource(RapierConfiguration {
            gravity: Vect::new(0.0, 0.0),
            ..Default::default()
        })
        .add_state::<GameState>()
        .add_state::<LevelState>()
        .add_systems(
            PostUpdate,
            watch_main_menu_event.run_if(in_state(GameState::Playing)),
        )
        .add_systems(PreUpdate, log_states);
        app.add_plugins(RapierDebugRenderPlugin::default());
    }
}

fn watch_main_menu_event(
    mut cmd: Commands,
    mut ev_main_menu: EventReader<MainMenuEvent>,
    //project_assets: Res<Assets<LdtkProject>>,
    q_project: Query<Entity, With<Handle<LdtkProject>>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut next_level_state: ResMut<NextState<LevelState>>,
) {
    for _ev in ev_main_menu.read() {
        if let Ok(entity) = q_project.get_single() {
            cmd.entity(entity).despawn_recursive();
            next_state.0 = Some(GameState::Menu);
            next_level_state.0 = Some(LevelState::WaitingLevelSpawn);
        }
    }
}

fn log_states(level_state: Res<State<LevelState>>, game_state: Res<State<GameState>>) {
    if game_state.is_changed() {
        info!("GameState: {:?}", *game_state);
    }
    if level_state.is_changed() {
        info!("LevelState: {:?}", *level_state);
    }
}
