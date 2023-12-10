use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_rapier2d::prelude::*;

const PLAYER_TINT: Color = Color::rgba(0.8, 0.8, 2., 1.0);
const ENEMY_TINT: Color = Color::rgba(1.5, 0.4, 0.4, 1.0);

#[derive(Component, Clone, Copy, Eq, PartialEq, Debug, Default)]
pub enum Team {
    #[default]
    Player,
    Enemy,
}

impl Team {
    pub fn opposing(self) -> Self {
        match self {
            Team::Player => Team::Enemy,
            Team::Enemy => Team::Player,
        }
    }

    pub fn tint(self) -> Color {
        match self {
            Team::Player => PLAYER_TINT,
            Team::Enemy => ENEMY_TINT,
        }
    }
}

pub struct ObjectGroup;

impl ObjectGroup {
    //pub const STEERING_SENSOR: u32 = 1 << 1;
    // pub const CLICK: u32 = 1 << 2;
    // pub const CLICK_SENSOR: u32 = 1 << 3;
    // pub const WALL: u32 = 1 << 4;
    // pub const ROBOT: u32 = 1 << 7;
    // pub const PLAYER: u32 = 1 << 8;
    // pub const ENEMY: u32 = 1 << 9;
    // pub const PORTAL_SENSOR: u32 = 1 << 10;
    pub const PLAYER_ROBOT: u32 = 1 << 1;
    pub const ENEMY_ROBOT: u32 = 1 << 2;
    pub const WALL: u32 = 1 << 3;
    pub const ROBOT_STEERING_SENSOR: u32 = 1 << 4;
    pub const PLAYER_PORTAL: u32 = 1 << 5;
    pub const ENEMY_PORTAL: u32 = 1 << 6;
    pub const PLAYER_ARROW_SENSOR: u32 = 1 << 7;
    pub const ENEMY_ARROW_SENSOR: u32 = 1 << 8;
    pub const PLAYER_PORTAL_SENSOR: u32 = 1 << 9;
    pub const ENEMY_PORTAL_SENSOR: u32 = 1 << 10;
    pub const CLICKABLE: u32 = 1 << 11;
    pub const PLAYER_TARGETING_SENSOR: u32 = 1 << 12;
    pub const ENEMY_TARGETING_SENSOR: u32 = 1 << 13;
    pub const PLAYER_BULLET: u32 = 1 << 14;
    pub const ENEMY_BULLET: u32 = 1 << 15;
}

pub fn coll_groups(members: u32, filters: u32) -> CollisionGroups {
    CollisionGroups::new(
        Group::from_bits_retain(members),
        Group::from_bits_retain(filters),
    )
}

#[derive(Default)]
pub struct CollisionCache {
    free_sets: Vec<HashSet<Entity>>,
    pub cache: HashMap<Entity, HashSet<Entity>>,
}

impl CollisionCache {
    pub fn cache_collisions(
        &mut self,
        events: &mut EventReader<CollisionEvent>,
        mut is_me: impl FnMut(Entity) -> bool,
    ) {
        for ev in events.read() {
            match ev {
                CollisionEvent::Started(e1, e2, _) => {
                    let (me, other) = if is_me(*e1) { (*e1, *e2) } else { (*e2, *e1) };
                    self.cache
                        .entry(me)
                        .or_insert_with(|| self.free_sets.pop().unwrap_or_default())
                        .insert(other);
                }
                CollisionEvent::Stopped(e1, e2, _) => {
                    let (me, other) = if is_me(*e1) { (*e1, *e2) } else { (*e2, *e1) };
                    if let Some(set) = self.cache.get_mut(&me) {
                        set.remove(&other);
                        if set.is_empty() {
                            self.free_sets.push(self.cache.remove(&me).unwrap());
                        }
                    }
                }
            }
        }
    }
}

// pub fn cache_collisions(
//     cache: &mut HashMap<Entity, HashSet<Entity>>,
//     events: &mut EventReader<CollisionEvent>,
//     mut is_me: impl FnMut(Entity) -> bool,
// ) {
//     for ev in events.read() {
//         match ev {
//             CollisionEvent::Started(e1, e2, _) => {
//                 let (me, other) = if is_me(*e1) { (*e1, *e2) } else { (*e2, *e1) };
//                 cache.entry(me).or_default().insert(other);
//             }
//             CollisionEvent::Stopped(e1, e2, _) => {
//                 let (me, other) = if is_me(*e1) { (*e1, *e2) } else { (*e2, *e1) };
//                 if let Some(set) = cache.get_mut(&me) {
//                     set.remove(&other);
//                     if set.is_empty() {
//                         cache.remove(&me);
//                     }
//                 }
//             }
//         }
//     }
// }
