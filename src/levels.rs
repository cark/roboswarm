use crate::arrow::EnemyArrowBundle;
use crate::game::GameState;
use crate::game_camera::{CameraStartBundle, CameraTargetPos};
use crate::game_ui::{ChangeLevelEvent, ResetLevelEvent};
use crate::inventory::Inventory;
use crate::physics::{coll_groups, ObjectGroup, Team};
use crate::portal::{EnemyPortalBundle, PlayerPortalBundle, Portal};
use bevy::math::{ivec2, vec2};
use bevy::prelude::*;
use bevy::utils::{info, HashMap};
use bevy_ecs_ldtk::prelude::*;
use bevy_ecs_ldtk::{assets::LdtkProject, LdtkWorldBundle};
use bevy_rapier2d::prelude::*;

// TODO: set level background color to transparent

//const GRID_SIZE: i32 = 16;
const AFTER_LOAD: GameState = GameState::Playing;
pub struct LevelsPlugin;

const LEVEL_NAMES: [&str; 2] = ["Level_0", "Level_1"];

impl Plugin for LevelsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LdtkAsset::default())
            .add_event::<LevelLoadedEvent>()
            .insert_resource(LevelIndex(0))
            .insert_resource(LevelCount(0))
            .insert_resource(LevelSelection::Identifier(LEVEL_NAMES[0].to_string()))
            .insert_resource(LevelSize::default())
            // .insert_resource(CurrentLevel::default())
            .add_systems(OnEnter(GameState::LoadingLevels), load_ldtk)
            .add_systems(
                Update,
                check_ldtk_loaded.run_if(in_state(GameState::LoadingLevels)),
            )
            .add_systems(OnEnter(GameState::Playing), spawn_ldtk)
            .add_systems(
                PostUpdate,
                level_changed.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (keep_level_on_screen.after(crate::mouse::motion),)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(Update, spawn_wall_collisions)
            .add_systems(
                Update,
                (check_victory, watch_for_reset, watch_for_next_level)
                    .run_if(in_state(GameState::Playing)),
            )
            .register_ldtk_int_cell_for_layer::<WallBundle>("Walls", 1)
            .register_ldtk_entity::<PlayerPortalBundle>("PlayerPortal")
            .register_ldtk_entity::<EnemyPortalBundle>("EnemyPortal")
            .register_ldtk_entity::<EnemyArrowBundle>("EnemyArrow")
            .register_ldtk_entity::<CameraStartBundle>("CameraStart");
    }
}

//#[derive(Resource, Default)]
// pub enum Victory {
//     #[default]
//     Undecided,

// }

#[derive(Resource)]
pub struct LevelIndex(pub usize);

#[derive(Resource)]
pub struct LevelCount(pub usize);

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Victory;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Loss;

#[derive(Resource, Default)]
struct LdtkAsset(Option<Handle<LdtkProject>>);

#[derive(Component)]
struct Ldtk;

#[derive(Clone, Debug, Default, Bundle, LdtkIntCell)]
pub struct WallColliderBundle {
    pub collider: Collider,
    pub ribid_body: RigidBody,
}

#[derive(Event)]
pub struct LevelLoadedEvent;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Component)]
pub struct Wall;

#[derive(Clone, Debug, Default, Bundle, LdtkIntCell)]
pub struct WallBundle {
    wall: Wall,
    //wall_collider_bundle: WallColliderBundle,
}

#[derive(Component)]
pub struct WallCollider;

#[derive(Resource, Default)]
pub struct LevelSize(pub Option<LevelSizeInfo>);

#[derive(Clone, Copy)]
pub struct LevelSizeInfo {
    pub size: IVec2,
    pub tile_size: i32,
}

#[derive(Component, Default)]
pub struct WallCache {
    pub items: HashMap<GridCoords, Entity>,
}

impl LevelSizeInfo {
    pub fn pixel_size(&self) -> IVec2 {
        self.size * self.tile_size
    }

    pub fn tile_size_vec(&self) -> IVec2 {
        IVec2::splat(self.tile_size)
    }

    pub fn grid_coords_in_bound(&self, grid_coords: GridCoords) -> bool {
        grid_coords.x >= 0
            && grid_coords.x < self.size.x
            && grid_coords.y >= 0
            && grid_coords.y < self.size.y
    }
}

fn load_ldtk(mut asset: ResMut<LdtkAsset>, asset_server: Res<AssetServer>) {
    asset.0 = Some(asset_server.load("levels.ldtk").clone());
}

fn check_ldtk_loaded(
    asset: Res<LdtkAsset>,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let handle = asset.0.clone().unwrap();
    if asset_server.is_loaded_with_dependencies(handle.clone()) {
        next_state.set(AFTER_LOAD);
    }
}
fn spawn_ldtk(mut cmd: Commands, asset: Res<LdtkAsset>) {
    let handle = asset.0.clone().unwrap();
    //let level_index = level_selection.as_ref().;
    cmd.spawn((
        LdtkWorldBundle {
            ldtk_handle: handle.clone(),
            transform: Transform::from_xyz(0.0, 0.0, -20.0),
            ..Default::default()
        },
        Ldtk,
    ));
}

fn level_changed(
    mut level_events: EventReader<LevelEvent>,
    project_assets: Res<Assets<LdtkProject>>,
    q_project: Query<&Handle<LdtkProject>>,
    mut q_level: Query<&mut Transform, With<LevelIid>>,
    mut ev_level_loaded: EventWriter<LevelLoadedEvent>,
    mut level_size: ResMut<LevelSize>,
    mut inventory: ResMut<Inventory>,
    mut level_count: ResMut<LevelCount>,
) {
    for level_event in level_events.read() {
        info(level_event);
        match level_event {
            LevelEvent::Spawned(level_uuid) => {
                let project = project_assets.get(q_project.single()).unwrap();
                level_count.0 = project.root_levels().iter().count();
                info!("level count : {}", level_count.0);
                let level = project
                    .as_standalone()
                    .get_loaded_level_by_iid(level_uuid.get())
                    .unwrap();
                let layer = &level.layer_instances()[0];
                let size_info = LevelSizeInfo {
                    size: ivec2(layer.c_wid, layer.c_hei),
                    tile_size: layer.grid_size,
                };
                for mut level_tr in &mut q_level {
                    level_tr.translation = (size_info.pixel_size().as_vec2() * -0.5).extend(-20.0);
                }
                inventory.arrow_count = *level.get_int_field("player_arrows").unwrap() as u32;

                level_size.0 = Some(size_info);
            }
            LevelEvent::Transformed(_) => {
                ev_level_loaded.send(LevelLoadedEvent);
            }
            _ => {}
        }
        //if let LevelEvent::Spawned(level_uuid) = level_event
    }
}

fn keep_level_on_screen(
    mut camera_target_pos: ResMut<CameraTargetPos>,
    q_level: Query<&Transform, With<LevelIid>>,
    level_size: Res<LevelSize>,
) {
    for tr in &q_level {
        if let Some(size_info) = level_size.0 {
            camera_target_pos.0 = camera_target_pos.0.clamp(
                tr.translation,
                tr.translation + size_info.pixel_size().as_vec2().extend(0.0),
            )
        }
    }
}

fn spawn_wall_collisions(
    mut cmd: Commands,
    q_wall: Query<(Entity, &GridCoords), With<Wall>>, // mut cmd: Commands,
    q_level: Query<Entity, With<LevelIid>>,
    level_size: Res<LevelSize>,
    mut ev_level_loaded: EventReader<LevelLoadedEvent>,
) {
    for _ in ev_level_loaded.read() {
        let mut wall_cache = HashMap::default();
        for (wall_entity, grid_coord) in &q_wall {
            if let Some(level_size) = level_size.0 {
                let half_size = level_size.tile_size as f32 / 2.0;
                cmd.entity(wall_entity).insert((
                    WallCollider,
                    Collider::cuboid(half_size + 1.0, half_size + 1.0),
                    RigidBody::Fixed,
                    Friction::new(0.0),
                    Restitution::new(2.0),
                    coll_groups(
                        ObjectGroup::WALL,
                        ObjectGroup::ENEMY_ROBOT
                            | ObjectGroup::PLAYER_ROBOT
                            | ObjectGroup::ROBOT_STEERING_SENSOR,
                    ),
                ));
                wall_cache.insert(*grid_coord, wall_entity);
            }
        }
        if let Ok(level_entity) = q_level.get_single() {
            cmd.entity(level_entity)
                .insert(WallCache { items: wall_cache });
        }
    }
}

fn check_victory(
    mut cmd: Commands,
    q_level: Query<Entity, (With<LevelIid>, Without<Victory>, Without<Loss>)>,
    q_portal: Query<&Team, With<Portal>>,
) {
    for e_level in &q_level {
        let mut player_portal_count = 0;
        let mut enemy_portal_count = 0;
        for team in &q_portal {
            match team {
                Team::Player => player_portal_count += 1,
                Team::Enemy => enemy_portal_count += 1,
            }
        }
        match (player_portal_count, enemy_portal_count) {
            (0, _) => {
                println!("loss");
                cmd.entity(e_level).insert(Loss);
            }
            (_, 0) => {
                println!("victory");
                cmd.entity(e_level).insert(Victory);
            }
            _ => {}
        }
    }
}

fn watch_for_reset(
    mut cmd: Commands,
    mut ev_reset_level: EventReader<ResetLevelEvent>,
    q_level: Query<Entity, With<LevelIid>>,
) {
    for _ev in ev_reset_level.read() {
        if let Ok(e_level) = q_level.get_single() {
            cmd.entity(e_level).try_insert(Respawn);
        }
    }
}

fn watch_for_next_level(
    mut cmd: Commands,
    mut ev_next_level: EventReader<ChangeLevelEvent>,
    mut level_selection: ResMut<LevelSelection>,
    mut level_index: ResMut<LevelIndex>,
) {
    for ev in ev_next_level.read() {
        match ev {
            ChangeLevelEvent::Next => level_index.0 += 1,
            ChangeLevelEvent::Previous => level_index.0 -= 1,
        }
        *level_selection = LevelSelection::Identifier(LEVEL_NAMES[level_index.0].to_string());
    }
}
