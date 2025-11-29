import super::common::{Path, Camera};
import super::queue::Queue;

@group(0) @binding(0)
var<storage, read_write> paths: array<Path>;

@group(1) @binding(0)
var<storage, read_write> new_path: Queue;

@group(2) @binding(0)
var<storage, read_write> extension: Queue;

@group(3) @binding(0)
var<uniform> camera: Camera;

@compute
@workgroup_size(64)
fn cs_main(
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    if atomicLoad(&new_path.read_counter) > arrayLength(&new_path.data) {
        // No work left to do, it overflowed
        return;
    }

    let i = atomicSub(&new_path.read_counter, 1);
    let path = paths[new_path.data[i]];
    if i < arrayLength(&new_path.data) && i > 0 {
        // new_path_queue[i] = global_id.x + global_id.y * 512;

    }
}
