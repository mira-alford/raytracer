use bevy_ecs::prelude::*;

use crate::{
    app::BevyApp,
    material::{MaterialId, MaterialServer},
    mesh::{MeshId, MeshServer},
    schedule,
    transform::Transform,
};

pub fn initialize(app: &mut BevyApp) {
    app.world
        .get_resource_or_init::<Schedules>()
        .add_systems(schedule::Update, binder_system);
}

fn binder_system(
    instances: Query<(&Transform, &MeshId, &MaterialId)>,
    mesh_server: Res<MeshServer>,
    material_server: Res<MaterialServer>,
) {
    for instance in &instances {}
    // Material server stores a series of buffers for each material,
    // same for mesh server (stores a series of buffers for the constructed BLASes)

    // tlas is constructed per frame? seems wasteful. Could store a tlas resource
    // and update it whenever a transform is changed!
}
