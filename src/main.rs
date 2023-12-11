#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// #[cfg(target_arch = "wasm32")]
use bevy::asset::AssetMetaCheck;

use bevy::{math::ivec2, prelude::*, window::WindowResolution};

use robo_swarm::*;

fn main() {
    let window = Window {
        title: "Bevy game".to_string(),
        // Bind to canvas included in `index.html`
        // canvas: Some("#bevy".to_owned()),
        // The canvas size is constrained in index.html and build/web/styles.css
        fit_canvas_to_parent: true,
        // Tells wasm not to override default event handling, like F5 and Ctrl+R
        prevent_default_event_handling: false,
        ..default()
    };

    #[cfg(all(debug_assertions, target_os = "windows"))]
    let window = Window {
        position: WindowPosition::At(ivec2(1920, 0)),
        resolution: WindowResolution::new(1900.0, 1024.0).with_scale_factor_override(1.0),
        ..window
    };

    let mut app = App::new();
    app.insert_resource(Msaa::Off)
        .insert_resource(ClearColor(Color::rgb(0.5, 0.1, 0.2)))
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(window),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
            GamePlugin,
        ));

    app.insert_resource(AssetMetaCheck::Never);
    // .add_systems(Startup, set_window_icon)
    app.run();
}
