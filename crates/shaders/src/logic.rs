use core::sync::atomic::AtomicU32;

use shared::glam::{UVec3, Vec2, Vec3};
use spirv_std::arch::{atomic_f_add, atomic_i_add};
use spirv_std::memory::{Scope, Semantics};
use spirv_std::spirv;

#[repr(C)]
#[derive(Default)]
pub struct Path {
    position: Vec3,
    _pad0: u32,
    direction: Vec3,
    _pad1: u32,
    terminated: u32,
    generated: u32,
    _pad2: Vec2,
    radiance: Vec3,
    _pad3: u32,
}

#[spirv(compute(threads(16, 16, 1)))]
pub fn cs_main(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] paths: &mut [Path],
    // New Path Queue:
    #[spirv(storage_buffer, descriptor_set = 1, binding = 0)] new_path_read: &mut u32,
    #[spirv(storage_buffer, descriptor_set = 1, binding = 1)] new_path_write: &mut u32,
    #[spirv(storage_buffer, descriptor_set = 1, binding = 2)] new_path_data: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 2, binding = 0)] output: &mut [u32],
) {
    // paths[id.x as usize] = Path::default();
    unsafe {
        atomic_i_add::<_, { Scope::Workgroup as u32 }, { Semantics::NONE.bits() }>(
            &mut output[(id.x + id.y * 512) as usize],
            1,
        );
    }
}

fn pack_rgb(color: Vec3) -> u32 {
    let r = ((color.x * 255.0) as u32) << 16;
    let g = ((color.y * 255.0) as u32) << 8;
    let b = (color.z * 255.0) as u32;
    return r | g | b;
}
