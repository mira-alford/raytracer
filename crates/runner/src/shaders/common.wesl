struct Path {
    position: vec3<f32>,
    direction: vec3<f32>,
    terminated: u32,
    generated: u32,
    radiance: vec3<f32>,
}

struct Camera {
    position: vec3<f32>,
    forward: vec3<f32>,
    up: vec3<f32>,
    right: vec3<f32>,
    focal_length: f32,
    logical_half_width: u32,
    logical_half_height: u32,
    world_half_width: f32,
    world_half_height: f32,
}
