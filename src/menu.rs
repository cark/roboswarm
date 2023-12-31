use bevy::{prelude::*, window::PrimaryWindow};

use crate::game::GameState;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ButtonState::None)
            .add_systems(OnEnter(GameState::Menu), init_menu)
            .add_systems(OnExit(GameState::Menu), destroy)
            .add_systems(PreUpdate, fix_font_sizes.run_if(in_state(GameState::Menu)))
            .add_systems(Update, button_system.run_if(in_state(GameState::Menu)));
    }
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.9, 0.9, 0.9);

#[derive(Resource)]
enum ButtonState {
    Down,
    None,
}

#[derive(Component)]
struct Menu;

#[derive(Component)]
struct TitleText;

#[derive(Component)]
struct ButtonText;

fn destroy(mut cmd: Commands, q: Query<Entity, With<Menu>>) {
    for e in &q {
        println!("destroy!");
        cmd.entity(e).despawn_recursive();
    }
}

fn button_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
    mut button_state: ResMut<ButtonState>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, mut color, mut border_color, children) in &mut interaction_query {
        if let Ok(mut text) = text_query.get_mut(children[0]) {
            let text = &mut text.sections[0];
            match *interaction {
                Interaction::Pressed => {
                    *color = PRESSED_BUTTON.into();
                    border_color.0 = Color::WHITE;
                    text.style.color = Color::BLACK;
                    *button_state = ButtonState::Down;
                }
                Interaction::Hovered => {
                    *color = HOVERED_BUTTON.into();
                    border_color.0 = Color::WHITE;
                    text.style.color = Color::rgb(0.9, 0.9, 0.9);
                    match *button_state {
                        ButtonState::Down => {
                            next_state.set(GameState::Playing);
                        }
                        _ => *button_state = ButtonState::None,
                    }
                }
                Interaction::None => {
                    *color = NORMAL_BUTTON.into();
                    border_color.0 = Color::BLACK;
                    text.style.color = Color::rgb(0.9, 0.9, 0.9);
                    *button_state = ButtonState::None;
                }
            }
        }
    }
}

fn fix_font_sizes(
    q_window: Query<&Window, With<PrimaryWindow>>,
    mut q_title_text: Query<&mut Text, (With<TitleText>, Without<ButtonText>)>,
    mut q_button_text: Query<&mut Text, (Without<TitleText>, With<ButtonText>)>,
) {
    let window = q_window.single();
    let height = window.height();
    for mut text in &mut q_title_text {
        text.sections[0].style.font_size = height / 6.;
    }
    for mut text in &mut q_button_text {
        text.sections[0].style.font_size = height / 10.;
    }
}

fn init_menu(mut cmd: Commands, asset_server: Res<AssetServer>) {
    cmd.spawn((
        Menu,
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceAround,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            ..Default::default()
        },
    ))
    .with_children(|cmd| {
        cmd.spawn((
            TitleText,
            TextBundle::from_section(
                "Roboswarm",
                TextStyle {
                    font: asset_server.load("GeoFont-Bold.otf"),
                    font_size: 60.,
                    color: Color::rgb(0.9, 0.9, 0.9),
                },
            ),
        ));
        cmd.spawn(ButtonBundle {
            style: Style {
                // width: Val::Px(150.0),
                // height: Val::Px(65.0),
                // padding: UiRect::new(Val::Px(20.), Val::Px(20.), Val::Px(15.), Val::Px(10.)),
                padding: UiRect::new(Val::VMax(2.), Val::VMax(2.), Val::VMax(1.5), Val::VMax(1.)),
                border: UiRect::all(Val::Px(5.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            border_color: BorderColor(Color::BLACK),
            background_color: NORMAL_BUTTON.into(),
            ..Default::default()
        })
        .with_children(|cmd| {
            cmd.spawn((
                ButtonText,
                TextBundle::from_section(
                    "Start",
                    TextStyle {
                        font: asset_server.load("GeoFont-Bold.otf"),
                        font_size: 40.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                    },
                ),
            ));
        });
    });
}
