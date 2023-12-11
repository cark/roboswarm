use std::time::Duration;

use bevy::{ecs::system::SystemChangeTick, prelude::*};

use crate::{
    arrow::DraggedArrow,
    defender::DraggedDefender,
    fork::DraggedFork,
    game::LevelState,
    game_camera::MouseWorldCoords,
    grouper::DraggedGrouper,
    inventory::Inventory,
    levels::{LevelCount, LevelIndex, LevelTitle, MaxAttainableLevel},
    mouse::{Drag, DragPos, MouseState},
};

pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ResetLevelEvent>()
            .add_event::<ChangeLevelEvent>()
            .add_event::<MainMenuEvent>()
            // .add_systems(OnEnter(GameState::Playing), instanciate)
            // .add_systems(OnExit(GameState::Playing), destroy)
            .add_systems(OnEnter(LevelState::Playing), instanciate_ui)
            .add_systems(OnExit(LevelState::Playing), destroy_ui)
            .add_systems(OnEnter(LevelState::Win), instanciate_win_screen)
            .add_systems(OnExit(LevelState::Win), destroy_win_screen)
            .add_systems(OnEnter(LevelState::Loss), instanciate_loss_screen)
            .add_systems(OnExit(LevelState::Loss), destroy_loss_screen)
            .add_systems(
                Update,
                (
                    button_system,
                    update_arrow_button,
                    update_fork_button,
                    update_grouper_button,
                    update_defender_button,
                    update_level_title,
                ),
            )
            .add_systems(Update, check_disabled.run_if(in_state(LevelState::Playing)))
            .add_systems(
                PostUpdate,
                (maybe_disable_previous_level, maybe_disable_next_level)
                    .run_if(in_state(LevelState::Playing)),
            );
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
struct ForkButton;
#[derive(Component)]
struct GrouperButton;
#[derive(Component)]
struct DefenderButton;

#[derive(Component)]
struct NextLevelButton;

#[derive(Component)]
struct PreviousLevelButton;

#[derive(Component)]
struct MainMenuButton;

#[derive(Component)]
enum ButtonType {
    Arrow,
    Fork,
    Grouper,
    Defender,
    Reset,
    NextLevel,
    PreviousLevel,
    MainMenu,
}

#[derive(Component)]
struct ArrowButtonText;
#[derive(Component)]
struct ForkButtonText;
#[derive(Component)]
struct GrouperButtonText;
#[derive(Component)]
struct DefenderButtonText;

#[derive(Event)]
pub struct ResetLevelEvent;

#[derive(Event)]
pub enum ChangeLevelEvent {
    Next,
    Previous,
}

#[derive(Event)]
pub struct MainMenuEvent;

#[derive(Component)]
struct LevelTitleText;
#[derive(Component)]
struct LevelTitleTextShadow;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ButtonDisabled;

#[derive(Component, Default)]
enum TitleState {
    #[default]
    Hidden,
    Display(Timer),
    Fade(Timer),
}

#[derive(Component)]
struct BaseColor(Color);

const TITLE_DISPLAY_TIME: Duration = Duration::from_millis(5000);
const TITLE_FADE_TIME: Duration = Duration::from_millis(10000);

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.9, 0.9, 0.9);
const DISABLED_BUTTON: Color = Color::rgb(0.3, 0.3, 0.3);

fn maybe_disable_next_level(
    mut cmd: Commands,
    level_count: Res<LevelCount>,
    level_index: Res<LevelIndex>,
    max_attainable_level: Res<MaxAttainableLevel>,
    q_next_level_button: Query<Entity, With<NextLevelButton>>,
    mut q_interaction: Query<&mut Interaction, With<Button>>,
) {
    if level_index.is_changed() || level_count.is_changed() {
        if let Ok(entity) = q_next_level_button.get_single() {
            // println!("{}", max_attainable_level.0);v
            if level_index.0 + 1 >= level_count.0 || level_index.0 >= max_attainable_level.0 {
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
    level_count: Res<LevelCount>,
    q_next_level_button: Query<Entity, With<PreviousLevelButton>>,
    mut q_interaction: Query<&mut Interaction, With<Button>>,
) {
    if level_index.is_changed() || level_count.is_changed() {
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

fn check_disabled(
    mut q_button: Query<(
        &Children,
        &mut BackgroundColor,
        &mut BorderColor,
        Option<&ButtonDisabled>,
    )>,
    mut q_visibility: Query<&mut Visibility>,
) {
    for (children, mut color, mut border_color, disabled) in &mut q_button {
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
        }
    }
}

fn button_system(
    mut cmd: Commands,
    mut interaction_query: Query<
        (
            Entity,
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
    mut ev_main_menu: EventWriter<MainMenuEvent>,
) {
    for (
        e_button,
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
        if disabled.is_none() {
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
                        ButtonState::Down => {
                            match button_type {
                                ButtonType::Arrow => {
                                    if inventory.arrow_count > 0
                                        && *mouse_state != MouseState::Dragging
                                    {
                                        cmd.spawn((
                                            Drag,
                                            DragPos(mouse_pos.0.unwrap()),
                                            DraggedArrow,
                                        ));
                                    }
                                }
                                ButtonType::Reset => ev_reset_level.send(ResetLevelEvent),
                                ButtonType::NextLevel => {
                                    // info!("send next level event from button {:?}", e_button);
                                    ev_next_level.send(ChangeLevelEvent::Next);
                                }
                                ButtonType::PreviousLevel => {
                                    ev_next_level.send(ChangeLevelEvent::Previous)
                                }
                                ButtonType::MainMenu => ev_main_menu.send(MainMenuEvent),
                                ButtonType::Fork => {
                                    if inventory.fork_count > 0
                                        && *mouse_state != MouseState::Dragging
                                    {
                                        cmd.spawn((
                                            Drag,
                                            DragPos(mouse_pos.0.unwrap()),
                                            DraggedFork,
                                        ));
                                    }
                                }
                                ButtonType::Grouper => {
                                    if inventory.grouper_count > 0
                                        && *mouse_state != MouseState::Dragging
                                    {
                                        cmd.spawn((
                                            Drag,
                                            DragPos(mouse_pos.0.unwrap()),
                                            DraggedGrouper,
                                        ));
                                    }
                                }
                                ButtonType::Defender => {
                                    if inventory.defender_count > 0 {
                                        cmd.spawn((
                                            Drag,
                                            DragPos(mouse_pos.0.unwrap()),
                                            DraggedDefender,
                                        ));
                                    }
                                }
                            }
                            *button_state = ButtonState::None;
                        }
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

fn destroy_ui(mut cmd: Commands, q_game_ui: Query<Entity, With<GameUi>>) {
    for entity in &q_game_ui {
        info!("Destroying game UI");
        cmd.entity(entity).despawn_recursive();
    }
}

fn instanciate_ui(mut cmd: Commands, asset_server: Res<AssetServer>) {
    info!("Instanciating game UI");
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
            cmd.spawn((
                MainMenuButton,
                ButtonState::None,
                ButtonType::MainMenu,
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
                        texture: asset_server.load("main_menu_button.png"),
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
        cmd.spawn((
            TextBundle::from_section(
                "Some level title here",
                TextStyle {
                    font: asset_server.load("GeoFont-Bold.otf"),
                    font_size: 48.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
                },
            ),
            LevelTitleText,
            TitleState::default(),
            BaseColor(Color::rgba(0.9, 0.9, 0.9, 1.0)),
        ))
        .with_children(|cmd| {
            let mut bundle = TextBundle::from_section(
                "Some level title here",
                TextStyle {
                    font: asset_server.load("GeoFont-Bold.otf"),
                    font_size: 48.0,
                    color: Color::rgba(0.0, 0.0, 0.0, 0.7),
                },
            );
            bundle.z_index = ZIndex::Global(-10);
            bundle.style.left = Val::Px(4.);
            bundle.style.top = Val::Px(4.);
            cmd.spawn((
                LevelTitleTextShadow,
                TitleState::default(),
                bundle,
                BaseColor(Color::rgba(0.0, 0.0, 0.0, 0.7)),
            ));
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
                    flex_direction: FlexDirection::Row,

                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|cmd| {
                spawn_placeable_button(
                    cmd,
                    &asset_server,
                    "arrow.png",
                    ButtonType::Arrow,
                    ArrowButton,
                    ArrowButtonText,
                );
                spawn_placeable_button(
                    cmd,
                    &asset_server,
                    "fork.png",
                    ButtonType::Fork,
                    ForkButton,
                    ForkButtonText,
                );
                spawn_placeable_button(
                    cmd,
                    &asset_server,
                    "grouper.png",
                    ButtonType::Grouper,
                    GrouperButton,
                    GrouperButtonText,
                );
                spawn_placeable_button(
                    cmd,
                    &asset_server,
                    "defender.png",
                    ButtonType::Defender,
                    DefenderButton,
                    DefenderButtonText,
                );
            });
        });
    });
}

//"fork.png"
//ButtonType::Fork
//ForkButton
//ForkButtonText
fn spawn_placeable_button<ButtonMarker: Component, TextMarker: Component>(
    cmd: &mut ChildBuilder,
    asset_server: &Res<AssetServer>,
    texture_name: &str,
    button_type: ButtonType,
    button_marker: ButtonMarker,
    text_marker: TextMarker,
) -> Entity {
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
        button_marker,
        button_type,
    ))
    .with_children(|cmd| {
        cmd.spawn(ImageBundle {
            image: UiImage {
                texture: asset_server.load(texture_name.to_string()),
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
                        text_marker,
                        TextBundle::from_section(
                            "0",
                            TextStyle {
                                font: asset_server.load("GeoFont-Bold.otf"),
                                font_size: 24.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                            },
                        ),
                    ));
                });
            });
        });
    })
    .id()
}

fn update_arrow_button(
    mut q_arrow_button_text: Query<&mut Text, With<ArrowButtonText>>,
    inventory: Res<Inventory>,
) {
    if let Ok(mut text) = q_arrow_button_text.get_single_mut() {
        text.sections[0].value = inventory.arrow_count.to_string();
    }
}

fn update_fork_button(
    mut q_fork_button_text: Query<&mut Text, With<ForkButtonText>>,
    inventory: Res<Inventory>,
) {
    if let Ok(mut text) = q_fork_button_text.get_single_mut() {
        text.sections[0].value = inventory.fork_count.to_string();
    }
}

fn update_grouper_button(
    mut q_grouper_button_text: Query<&mut Text, With<GrouperButtonText>>,
    inventory: Res<Inventory>,
) {
    if let Ok(mut text) = q_grouper_button_text.get_single_mut() {
        text.sections[0].value = inventory.grouper_count.to_string();
    }
}
fn update_defender_button(
    mut q_defender_button_text: Query<&mut Text, With<DefenderButtonText>>,
    inventory: Res<Inventory>,
) {
    if let Ok(mut text) = q_defender_button_text.get_single_mut() {
        text.sections[0].value = inventory.defender_count.to_string();
    }
}

fn update_level_title(
    mut q_title: Query<(&mut Text, &mut TitleState, &BaseColor)>,
    level_title: Res<LevelTitle>,
    time: Res<Time>,
) {
    for (mut text, mut title_state, base_color) in &mut q_title {
        let section = &mut text.sections[0];
        if level_title.0 != section.value {
            section.value = level_title.0.clone();
            *title_state = TitleState::Display(Timer::new(TITLE_DISPLAY_TIME, TimerMode::Once));
        }
        match *title_state {
            TitleState::Hidden => section.style.color = Color::rgba(0.0, 0.0, 0.0, 0.0),
            TitleState::Display(ref mut timer) => {
                timer.tick(time.delta());
                if timer.finished() {
                    *title_state = TitleState::Fade(Timer::new(TITLE_FADE_TIME, TimerMode::Once));
                } else {
                    section.style.color = base_color.0;
                }
            }
            TitleState::Fade(ref mut timer) => {
                timer.tick(time.delta());
                if timer.finished() {
                    *title_state = TitleState::Hidden;
                } else {
                    let alpha = (timer.duration().as_secs_f32() - timer.elapsed_secs())
                        / timer.duration().as_secs_f32();
                    let alpha = ((alpha + 0.01).log10() * 10.).exp().clamp(0.0, 1.0);
                    section.style.color.set_a(base_color.0.a() * alpha);
                }
            }
        }
    }
}

#[derive(Component)]
struct WinScreen;

fn destroy_win_screen(mut cmd: Commands, q_win_screen: Query<Entity, With<WinScreen>>) {
    for entity in &q_win_screen {
        info!("Destroying win screen");
        cmd.entity(entity).despawn_recursive();
    }
}

fn instanciate_win_screen(
    mut cmd: Commands,
    asset_server: Res<AssetServer>,
    level_count: Res<LevelCount>,
    level_index: Res<LevelIndex>,
) {
    info!("instanciating win screen");
    cmd.spawn((
        WinScreen,
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
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
            // background_color: Color::GREEN.into(),
            ..Default::default()
        },
    ))
    .with_children(|cmd| {
        cmd.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                padding: UiRect {
                    top: Val::Vh(2.),
                    left: Val::Vh(2.),
                    right: Val::Vh(2.),
                    bottom: Val::Vh(2.),
                },
                ..Default::default()
            },
            background_color: Color::rgba(0.0, 0.0, 0.0, 0.8).into(),
            ..Default::default()
        })
        .with_children(|cmd| {
            cmd.spawn(NodeBundle {
                style: Style {
                    padding: UiRect {
                        top: Val::Vh(2.),
                        left: Val::Vh(20.),
                        right: Val::Vh(20.),
                        bottom: Val::Vh(2.),
                    },
                    ..Default::default()
                },
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.8).into(),
                ..Default::default()
            })
            .with_children(|cmd| {
                let text_bundle = TextBundle::from_section(
                    if level_index.0 + 1 >= level_count.0 {
                        "You won the game !"
                    } else {
                        "Victory !"
                    },
                    TextStyle {
                        font: asset_server.load("GeoFont-Bold.otf"),
                        font_size: 48.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                    },
                );
                cmd.spawn(text_bundle);
            });
            cmd.spawn(NodeBundle {
                style: Style {
                    padding: UiRect {
                        top: Val::Vh(2.),
                        left: Val::Vh(20.),
                        right: Val::Vh(20.),
                        bottom: Val::Vh(2.),
                    },
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    ..Default::default()
                },
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.8).into(),
                ..Default::default()
            })
            .with_children(|cmd| {
                cmd.spawn((
                    MainMenuButton,
                    ButtonState::None,
                    ButtonType::MainMenu,
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
                            texture: asset_server.load("main_menu_button.png"),
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
                if level_index.0 + 1 < level_count.0 {
                    cmd.spawn((
                        NextLevelButton,
                        ButtonState::None,
                        ButtonType::NextLevel,
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
                }
            });
        });
    });
}

#[derive(Component)]
struct LossScreen;

fn destroy_loss_screen(mut cmd: Commands, q_loss_screen: Query<Entity, With<LossScreen>>) {
    for entity in &q_loss_screen {
        info!("Destroying loss screen");
        cmd.entity(entity).despawn_recursive();
    }
}

fn instanciate_loss_screen(mut cmd: Commands, asset_server: Res<AssetServer>) {
    info!("instanciating loss screen");
    cmd.spawn((
        LossScreen,
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
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
            // background_color: Color::GREEN.into(),
            ..Default::default()
        },
    ))
    .with_children(|cmd| {
        cmd.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                padding: UiRect {
                    top: Val::Vh(2.),
                    left: Val::Vh(2.),
                    right: Val::Vh(2.),
                    bottom: Val::Vh(2.),
                },
                ..Default::default()
            },
            background_color: Color::rgba(0.0, 0.0, 0.0, 0.8).into(),
            ..Default::default()
        })
        .with_children(|cmd| {
            cmd.spawn(NodeBundle {
                style: Style {
                    padding: UiRect {
                        top: Val::Vh(2.),
                        left: Val::Vh(20.),
                        right: Val::Vh(20.),
                        bottom: Val::Vh(2.),
                    },
                    ..Default::default()
                },
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.8).into(),
                ..Default::default()
            })
            .with_children(|cmd| {
                let text_bundle = TextBundle::from_section(
                    "Defeat...",
                    TextStyle {
                        font: asset_server.load("GeoFont-Bold.otf"),
                        font_size: 48.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                    },
                );
                cmd.spawn(text_bundle);
            });
            cmd.spawn(NodeBundle {
                style: Style {
                    padding: UiRect {
                        top: Val::Vh(2.),
                        left: Val::Vh(20.),
                        right: Val::Vh(20.),
                        bottom: Val::Vh(2.),
                    },
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    ..Default::default()
                },
                background_color: Color::rgba(0.0, 0.0, 0.0, 0.8).into(),
                ..Default::default()
            })
            .with_children(|cmd| {
                cmd.spawn((
                    MainMenuButton,
                    ButtonState::None,
                    ButtonType::MainMenu,
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
                            texture: asset_server.load("main_menu_button.png"),
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
            });
        });
    });
}
