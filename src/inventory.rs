use bevy::prelude::*;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Inventory::default());
        //todo!()
    }
}

#[derive(Default, Resource)]
pub struct Inventory {
    pub arrow_count: u32,
    pub fork_count: u32,
}
