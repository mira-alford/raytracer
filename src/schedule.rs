use bevy_ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Clone, Eq, PartialEq, Debug, Hash)]
pub struct Startup;

#[derive(ScheduleLabel, Clone, Eq, PartialEq, Debug, Hash)]
pub struct PreStartup;

#[derive(ScheduleLabel, Clone, Eq, PartialEq, Debug, Hash)]
pub struct Update;
