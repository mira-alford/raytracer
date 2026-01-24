use bevy_ecs::component::Component;
use glam::Vec4;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default, Component)]
pub struct Transform {
    pub scale: Vec4,
    pub rotation: Vec4,
    pub translation: Vec4,
}
