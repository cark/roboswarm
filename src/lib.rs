#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

mod arrow;
mod bullet;
mod defender;
mod draggable;
mod explosion;
mod fork;
mod game;
mod game_camera;
mod game_ui;
mod grouper;
mod hp;
mod inventory;
mod levels;
mod load;
mod menu;
mod mouse;
mod physics;
mod portal;
mod robot;

pub use game::GamePlugin;
