use crate::game::GameState;
use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

pub struct LoadPlugin;

impl Plugin for LoadPlugin {
    fn build(&self, app: &mut App) {
        // app.add_systems(Startup, load_images);
        app.add_loading_state(
            LoadingState::new(GameState::Loading).continue_to_state(GameState::LoadingLevels),
        )
        .add_collection_to_loading_state::<_, TextureAssets>(GameState::Loading);
        // .add_collection_to_loading_state::<_, LevelAssets>(GameState::Loading);
    }
}

#[derive(AssetCollection, Resource)]
pub struct TextureAssets {
    #[asset(path = "robot_body.png")]
    pub robot_body: Handle<Image>,
    #[asset(path = "robot_canon.png")]
    pub robot_canon: Handle<Image>,
    #[asset(path = "robot_turret.png")]
    pub robot_turret: Handle<Image>,
    #[asset(path = "robot_train.png")]
    pub robot_train: Handle<Image>,
    #[asset(path = "enemy_portal.png")]
    pub enemy_portal: Handle<Image>,
    #[asset(path = "player_portal.png")]
    pub player_portal: Handle<Image>,
    #[asset(path = "arrow.png")]
    pub arrow: Handle<Image>,
    #[asset(path = "nuzzle_flash.png")]
    pub nuzzle_flash: Handle<Image>,
    #[asset(path = "bullet.png")]
    pub bullet: Handle<Image>,
    #[asset(path = "explosion_particle.png")]
    pub explosion_particle: Handle<Image>,
}
