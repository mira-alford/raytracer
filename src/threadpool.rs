use bevy_ecs::prelude::*;

use crate::{app::BevyApp, schedule};

#[derive(Resource)]
pub struct ThreadPool(pub rayon::ThreadPool);

pub fn initialize(app: &mut BevyApp) {
    app.world
        .get_resource_or_init::<Schedules>()
        .add_systems(schedule::PreStartup, setup_threadpool);
}

fn setup_threadpool(mut commands: Commands) {
    commands.insert_resource(ThreadPool(
        rayon::ThreadPoolBuilder::new()
            .build()
            .expect("Expected a threadpool"),
    ));
}
