use bevy::{math::vec2, prelude::*};
use bevy_ecs_ldtk::{
    prelude::*,
    utils::{grid_coords_to_translation, translation_to_grid_coords},
};

use crate::{
    game_camera::MouseWorldCoords,
    levels::{LevelSize, NoPlacingHere, WallCache},
    load::TextureAssets,
    mouse::{Drag, DragCancelConfirm, DragCancelRequest, DragDropRequest, DragPos},
};

#[derive(Component, PartialEq)]
pub enum DragState {
    Dragging,
    SettingDirection(Transform),
}

#[derive(Component)]
pub struct ValidDrag;

pub fn validate_drag<DraggedMarker: Component>(
    mut cmd: Commands,
    mut q_drag: Query<(Entity, &mut Transform, &mut Sprite, &DragState), With<DraggedMarker>>,
    mouse_pos: Res<MouseWorldCoords>,
    q_level: Query<(&GlobalTransform, &WallCache), With<LevelIid>>,
    level_size: Res<LevelSize>,
    q_occupied: Query<&GridCoords, With<NoPlacingHere>>,
) {
    for (entity, mut drag_tr, mut sprite, drag_state) in &mut q_drag {
        match drag_state {
            DragState::Dragging => {
                cmd.entity(entity).remove::<ValidDrag>();
                if let Some(pos) = mouse_pos.0 {
                    drag_tr.translation = pos.extend(0.0);
                    sprite.color = Color::WHITE.with_a(0.4);
                    let (level_gtr, wall_cache) = q_level.single();
                    if let Some(size_info) = level_size.0 {
                        let coords = translation_to_grid_coords(
                            drag_tr.translation.truncate() - level_gtr.translation().truncate(),
                            size_info.tile_size_vec(),
                        );
                        if size_info.grid_coords_in_bound(coords)
                            && !wall_cache.items.contains_key(&coords)
                            && q_occupied.iter().all(|grid_coord| {
                                (Into::<IVec2>::into(*grid_coord) - Into::<IVec2>::into(coords))
                                    .as_vec2()
                                    .length()
                                    >= 2.0
                            })
                        {
                            drag_tr.translation =
                                grid_coords_to_translation(coords, size_info.tile_size_vec())
                                    .extend(0.0)
                                    + level_gtr
                                        .translation()
                                        .truncate()
                                        .extend(drag_tr.translation.z);
                            sprite.color = Color::WHITE.with_a(1.0);
                            cmd.entity(entity).insert(ValidDrag).insert(coords);
                        }
                    }
                }
            }
            DragState::SettingDirection(center_tr) => {
                if let Some(pos) = mouse_pos.0 {
                    let angle =
                        vec2(1.0, 0.0).angle_between(pos - center_tr.translation.truncate());
                    *drag_tr = drag_tr.with_rotation(Quat::from_rotation_z(angle));
                }
            }
        }
    }
}

pub fn draggable_spawner<DraggedMarker: Component>(
    texture_name: &'static str,
) -> impl Fn(Commands, Res<AssetServer>, Query<(Entity, &DragPos), (With<DraggedMarker>, Added<Drag>)>)
{
    move |mut cmd: Commands,
          assets: Res<AssetServer>,
          q_drag: Query<(Entity, &DragPos), (With<DraggedMarker>, Added<Drag>)>| {
        for (entity, drag_pos) in &q_drag {
            cmd.entity(entity).insert((
                SpriteBundle {
                    transform: Transform::from_translation(drag_pos.0.extend(0.0)),
                    texture: assets.load(texture_name).clone(),
                    ..Default::default()
                },
                DragState::Dragging,
            ));
        }
    }
}

pub fn drag_cancel_request<DraggedMarker: Component>(
    mut cmd: Commands,
    q_drag: Query<Entity, (Added<DragCancelRequest>, With<DraggedMarker>)>,
) {
    for entity in &q_drag {
        cmd.entity(entity).insert(DragCancelConfirm);
    }
}
