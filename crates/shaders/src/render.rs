use shared::glam::Vec3;
use spirv_std::glam::{Vec2, Vec4};
use spirv_std::image::Image2d;
use spirv_std::{Sampler, spirv};

#[allow(unused_attributes)]
#[spirv(vertex)]
pub fn vs_main(
    in_position: Vec3,
    in_uv: Vec2,
    #[spirv(position)] out_pos: &mut Vec4,
    out_uv: &mut Vec2,
) {
    *out_uv = in_uv;
    *out_pos = Vec4::new(in_position.x, in_position.y, in_position.z, 1.0);
}

#[allow(unused_attributes)]
#[spirv(fragment)]
pub fn fs_main(
    in_tex_coords: Vec2,
    #[spirv(descriptor_set = 0, binding = 0)] t_diffuse: &Image2d,
    #[spirv(descriptor_set = 0, binding = 1)] s_diffuse: &Sampler,
    output: &mut Vec4,
) {
    *output = t_diffuse.sample(*s_diffuse, in_tex_coords);
}
