import super::common::{Path};
import super::queue::{Queue, queue_push};

@group(0) @binding(0)
var<storage, read_write> paths: array<Path>;

@group(1) @binding(0)
var<storage, read_write> new_path: Queue;

@group(2) @binding(0)
var<storage, read_write> output: array<atomic<u32>>;

@compute
@workgroup_size(16,16,1)
fn cs_main(
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    if global_id.x > 512 || global_id.y > 512 {
        return;
    }

    let path = &paths[global_id.x + global_id.y * 512];
    if path.generated == 0 {
        path.generated = 1;
        // queue_push(&new_path, global_id.x + global_id.y * 512);
        // atomicStore(&output[global_id.x + global_id.y * 512], i);
    } else if path.terminated == 1 {
        // On termination, add the radiance to the buffer for now:
        atomicAdd(&output[global_id.x + global_id.y * 512], pack_rgb(path.radiance));
    } else {
    }
}

fn pack_rgb(color: vec3<f32>) -> u32 {
    let r = u32(color.r * 255.0) << 16;
    let g = u32(color.g * 255.0) << 8;
    let b = u32(color.b * 255.0);
    return r | g | b;
}
