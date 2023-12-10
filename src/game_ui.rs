use bevy::prelude::*;

use crate::{
    arrow::DraggedArrow,
    game::GameState,
    game_camera::MouseWorldCoords,
    inventory::Inventory,
    levels::{LevelCount, LevelIndex},
    mouse::{Drag, DragPos, MouseState},
};

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ResetLevelEvent>()
            .add_event::<ChangeLevelEvent>()
            .add_systems(OnEnter(GameState::Playing), instanciate)
            .add_systems(
                Update,
                (
                    button_system,
                    update_arrow_button,
                    (maybe_disable_previous_level, maybe_disable_next_level).after(button_system),
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn maybe_disable_next_level(
    mut cmd: Commands,
    level_count: Res<LevelCount>,
    level_index: Res<LevelIndex>,
    q_next_level_button: Query<Entity, With<NextLevelButton>>,
    mut q_interaction: Query<&mut Interaction, With<Button>>,
) {
    if level_index.is_changed() || level_count.is_changed() {
        if let Ok(entity) = q_next_level_button.get_single() {
            if level_index.0 + 1 >= level_count.0 {
                cmd.entity(entity).try_insert(ButtonDisabled);
            } else {
                cmd.entity(entity).remove::<ButtonDisabled>();
            }
            if let Ok(mut interaction) = q_interaction.get_mut(entity) {
                interaction.set_changed();
            }
        }
    }
}

fn maybe_disable_previous_level(
    mut cmd: Commands,
    level_index: Res<LevelIndex>,
    q_next_level_button: Query<Entity, With<PreviousLevelButton>>,
    mut q_interaction: Query<&mut Interaction, With<Button>>,
) {
    if level_index.is_changed() {
        if let Ok(entity) = q_next_level_button.get_single() {
            if level_index.0 == 0 {
                cmd.entity(entity).try_insert(ButtonDisabled);
            } else {
                cmd.entity(entity).remove::<ButtonDisabled>();
            }
            if let Ok(mut interaction) = q_interaction.get_mut(entity) {
                interaction.set_changed();
            }
        }
    }
}

#[derive(Component)]
struct GameUi;

#[derive(Component)]
enum ButtonState {
    Down,
    None,
}

#[derive(Component)]
struct ArrowButton;

#[derive(Component)]
struct NextLevelButton;

#[derive(Component)]
struct PreviousLevelButton;

#[derive(Component)]
enum ButtonType {
    Arrow,
    Reset,
    NextLevel,
    PreviousLevel,
}

#[derive(Component)]
struct ArrowButtonText;

#[derive(Event)]
pub struct ResetLevelEvent;

#[derive(Event)]
pub enum ChangeLevelEvent {
    Next,
    Previous,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ButtonDisabled;

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.9, 0.9, 0.9);
const DISABLED_BUTTON: Color = Color::rgb(0.3, 0.3, 0.3);

fn button_system(
    mut cmd: Commands,
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut ButtonState,
            &ButtonType,
            Option<&ButtonDisabled>,
            &Children,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut q_visibility: Query<&mut Visibility>,
    inventory: Res<Inventory>,
    mouse_pos: Res<MouseWorldCoords>,
    mouse_state: Res<MouseState>,
    mut ev_reset_level: EventWriter<ResetLevelEvent>,
    mut ev_next_level: EventWriter<ChangeLevelEvent>,
) {
    for (
        interaction,
        mut color,
        mut border_color,
        mut button_state,
        button_type,
        disabled,
        children,
    ) in &mut interaction_query
    {
        //let text = &mut text_query.get_mut(children[0]).unwrap().sections[0];
        if disabled.is_some() {
            *color = DISABLED_BUTTON.into();
            border_color.0 = DISABLED_BUTTON;
            for child in children.iter() {
                if let Ok(mut visibility) = q_visibility.get_mut(*child) {
                    *visibility = Visibility::Hidden;
                }
            }
        } else {
            for child in children.iter() {
                if let Ok(mut visibility) = q_visibility.get_mut(*child) {
                    *visibility = Visibility::Visible;
                }
            }
            match *interaction {
                Interaction::Pressed => {
                    *color = PRESSED_BUTTON.into();
                    border_color.0 = Color::WHITE;
                    //text.style.color = Color::BLACK;
                    *button_state = ButtonState::Down;
                }
                Interaction::Hovered => {
                    *color = HOVERED_BUTTON.into();
                    border_color.0 = Color::WHITE;
                    //text.style.color = Color::rgb(0.9, 0.9, 0.9);
                    match *button_state {
                        ButtonState::Down => match button_type {
                            ButtonType::Arrow => {
                                if inventory.arrow_count > 0 && *mouse_state != MouseState::Dragging
                                {
                                    cmd.spawn((Drag, DragPos(mouse_pos.0.unwrap()), DraggedArrow));
                                }
                            }
                            ButtonType::Reset => ev_reset_level.send(ResetLevelEvent),
                            ButtonType::NextLevel => ev_next_level.send(ChangeLevelEvent::Next),
                            ButtonType::PreviousLevel => {
                                ev_next_level.send(ChangeLevelEvent::Previous)
                            }
                        },
                        _ => *button_state = ButtonState::None,
                    }
                }
                Interaction::None => {
                    *color = NORMAL_BUTTON.into();
                    border_color.0 = Color::BLACK;
                    //text.style.color = Color::rgb(0.9, 0.9, 0.9);
                    *button_state = ButtonState::None;
                }
            }
        }
    }
}

fn instanciate(mut cmd: Commands, asset_server: Res<AssetServer>) {
    cmd.spawn((
        GameUi,
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                padding: UiRect {
                    top: Val::Vh(2.),
                    left: Val::Vh(2.),
                    right: Val::Vh(2.),
                    bottom: Val::Vh(2.),
                },
                ..Default::default()
            },
            ..Default::default()
        },
    ))
    .with_children(|cmd| {
        cmd.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::FlexStart,
                flex_direction: FlexDirection::Row,
                ..Default::default()
            },
            //background_color: Color::GREEN.into(),
            ..Default::default()
        })
        .with_children(|cmd| {
            cmd.spawn((
                ButtonState::None,
                ButtonType::Reset,
                ButtonBundle {
                    style: Style {
                        width: Val::VMin(7.),
                        height: Val::VMin(7.),
                        border: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    border_color: BorderColor(Color::BLACK),
                    background_color: NORMAL_BUTTON.into(),
                    ..Default::default()
                },
            ))
            .with_children(|cmd| {
                cmd.spawn(ImageBundle {
                    image: UiImage {
                        texture: asset_server.load("reset_button.png"),
                        ..Default::default()
                    },
                    style: Style {
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    ..Default::default()
                });
            });
            cmd.spawn((
                PreviousLevelButton,
                ButtonState::None,
                ButtonType::PreviousLevel,
                ButtonBundle {
                    style: Style {
                        width: Val::VMin(7.),
                        height: Val::VMin(7.),
                        border: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    border_color: BorderColor(Color::BLACK),
                    background_color: NORMAL_BUTTON.into(),
                    ..Default::default()
                },
            ))
            .with_children(|cmd| {
                cmd.spawn(ImageBundle {
                    image: UiImage {
                        texture: asset_server.load("previous_level_button.png"),
                        ..Default::default()
                    },
                    style: Style {
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    ..Default::default()
                });
            });
            cmd.spawn((
                NextLevelButton,
                ButtonState::None,
                ButtonType::NextLevel,
                // ButtonDisabled,
                ButtonBundle {
                    style: Style {
                        width: Val::VMin(7.),
                        height: Val::VMin(7.),
                        border: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    border_color: BorderColor(Color::BLACK),
                    background_color: NORMAL_BUTTON.into(),
                    ..Default::default()
                },
            ))
            .with_children(|cmd| {
                cmd.spawn(ImageBundle {
                    image: UiImage {
                        texture: asset_server.load("next_level_button.png"),
                        ..Default::default()
                    },
                    style: Style {
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    ..Default::default()
                });
            });
        });
        cmd.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::FlexEnd,
                flex_direction: FlexDirection::Row,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|cmd| {
            cmd.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,

                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|cmd| {
                cmd.spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::VMin(8.),
                            height: Val::VMin(8.),
                            border: UiRect::all(Val::Px(5.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..Default::default()
                        },
                        border_color: BorderColor(Color::BLACK),
                        background_color: NORMAL_BUTTON.into(),
                        ..Default::default()
                    },
                    ButtonState::None,
                    ArrowButton,
                    ButtonType::Arrow,
                ))
                .with_children(|cmd| {
                    cmd.spawn(ImageBundle {
                        image: UiImage {
                            texture: asset_server.load("arrow.png"),
                            ..Default::default()
                        },
                        style: Style {
                            width: Val::Percent(80.),
                            height: Val::Percent(80.),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|cmd| {
                        cmd.spawn(NodeBundle {
                            style: Style {
                                height: Val::Percent(100.),
                                width: Val::Percent(100.),
                                align_items: AlignItems::FlexEnd,
                                ..Default::default()
                            },
                            // background_color: BackgroundColor(Color::BLUE),
                            ..Default::default()
                        })
                        .with_children(|cmd| {
                            cmd.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Row,
                                    justify_content: JustifyContent::FlexEnd,
                                    align_items: AlignItems::FlexEnd,
                                    ..Default::default()
                                },
                                // background_color: BackgroundColor(Color::RED),
                                ..Default::default()
                            })
                            .with_children(|cmd| {
                                cmd.spawn((
                                    TextBundle::from_section(
                                        "0",
                                        TextStyle {
                                            font: asset_server.load("GeoFont-Bold.otf"),
                                            font_size: 24.0,
                                            color: Color::rgb(0.9, 0.9, 0.9),
                                        },
                                    ),
                                    ArrowButtonText,
                                ));
                            });
                        });
                    });
                });
            });
        });
    });
}

fn update_arrow_button(
    mut q_arrow_button_text: Query<&mut Text, With<ArrowButtonText>>,
    inventory: Res<Inventory>,
) {
    if inventory.is_changed() {
        if let Ok(mut text) = q_arrow_button_text.get_single_mut() {
            text.sections[0].value = inventory.arrow_count.to_string();
        }
    }
}
