use bevy_ecs::prelude::*;
use wesl::include_wesl;
use wgpu::{CommandBuffer, include_spirv, util::DeviceExt};

use crate::{
    app::BevyApp,
    binder::{SceneBindings, binder_system},
    camera::Camera,
    pathtracer::{Pathtracer, PathtracerOutput, pathtracer_output_sync_system},
    pathtracer_state::PathtracerState,
    render::render_system,
    render_resources::{RenderDevice, RenderQueue, RenderSurface},
    schedule,
};

#[derive(Component)]
pub struct PathtracerPhase {
    sample_main_pipeline: wgpu::ComputePipeline,
    sample_cleanup_pipeline: wgpu::ComputePipeline,
    ray_extend_pipeline: wgpu::ComputePipeline,
}

pub fn initialize(app: &mut BevyApp) {
    app.world.get_resource_or_init::<Schedules>().add_systems(
        schedule::Update,
        (
            pathtracer_phase_execute
                .before(render_system)
                .after(binder_system),
            pathtracer_phase_sync
                .before(pathtracer_phase_execute)
                .after(pathtracer_output_sync_system)
                .after(binder_system),
        ),
    );
}

fn pathtracer_phase_sync(
    pathtracer_query: Query<
        (
            Entity,
            &Pathtracer,
            &PathtracerOutput,
            Option<&mut PathtracerState>,
            Option<&mut PathtracerPhase>,
            &Camera,
        ), // add camera component here pls :)
        Changed<PathtracerOutput>,
    >,
    mut commands: Commands,
    device: Res<RenderDevice>,
    scene_bindings: Res<SceneBindings>,
) {
    // Update all the path tracer states to be reset:
    for (e, pt, pto, pts, ptp, camera) in pathtracer_query {
        let new_pts = PathtracerState::new(&device.0, pt.dims, pt.threads);
        let new_ptp = PathtracerPhase::new(&device.0, &pto, &scene_bindings, &new_pts, camera);

        if let Some(mut pts) = pts {
            *pts = new_pts;
        } else {
            commands.entity(e).insert(new_pts);
        }

        if let Some(mut ptp) = ptp {
            *ptp = new_ptp;
        } else {
            commands.entity(e).insert(new_ptp);
        }
    }
}

fn pathtracer_phase_execute(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    query: Query<(
        &Pathtracer,
        &PathtracerOutput,
        &PathtracerState,
        &PathtracerPhase,
        &Camera,
    )>,
    scene_bindings: Res<SceneBindings>,
) {
    if scene_bindings.bind_group.is_none() {
        return;
    }

    for (pt, pto, pts, ptp, camera) in query {
        let mut encoder = device
            .0
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&ptp.sample_cleanup_pipeline);
        compute_pass.set_bind_group(0, scene_bindings.bind_group.as_ref().unwrap(), &[]);
        compute_pass.set_bind_group(1, &pts.bind_group, &[]);
        compute_pass.set_bind_group(2, &camera.bind_group, &[]);
        compute_pass.set_bind_group(3, &pto.source_bind_group, &[]);
        compute_pass.dispatch_workgroups(4096.min((pt.dims.0 * pt.dims.1).div_ceil(64)), 1, 1);

        compute_pass.set_pipeline(&ptp.sample_main_pipeline);
        compute_pass.dispatch_workgroups(pt.threads.div_ceil(64), 1, 1);

        compute_pass.set_pipeline(&ptp.ray_extend_pipeline);
        compute_pass.dispatch_workgroups(pt.threads.div_ceil(64), 1, 1);

        drop(compute_pass);

        let command = encoder.finish();

        queue.0.submit([command]);
    }
}

// pub fn render_system(
//     device: Res<RenderDevice>,
//     queue: Res<RenderQueue>,
//     query: Query<(&Pathtracer, &PathtracerOutput)>,
//     surface: Res<RenderSurface>,
//     render_phase: If<Res<RenderPhase>>,
// ) {
//     for (pt, pto) in query {
//         if !pt.is_primary {
//             continue;
//         }

//         let mut encoder = device
//             .0
//             .create_command_encoder(&wgpu::CommandEncoderDescriptor {
//                 label: Some("Render Encoder"),
//             });

//         pto.copy_to_texture(&mut encoder);

//         let surface_texture = surface.surface.get_current_texture().unwrap();
//         let surface_view = surface_texture
//             .texture
//             .create_view(&wgpu::TextureViewDescriptor::default());

//         let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//             label: Some("Render Pass"),
//             color_attachments: &[Some(wgpu::RenderPassColorAttachment {
//                 view: &surface_view,
//                 resolve_target: None,
//                 ops: wgpu::Operations {
//                     load: wgpu::LoadOp::Clear(wgpu::Color {
//                         r: 0.1,
//                         g: 0.2,
//                         b: 0.3,
//                         a: 1.0,
//                     }),
//                     store: wgpu::StoreOp::Store,
//                 },
//                 depth_slice: None,
//             })],
//             depth_stencil_attachment: None,
//             occlusion_query_set: None,
//             timestamp_writes: None,
//         });

//         render_pass.set_pipeline(&render_phase.render_pipeline);
//         render_pass.set_bind_group(0, &render_phase.bind_group, &[]);
//         render_pass.set_vertex_buffer(0, render_phase.vertex_buffer.slice(..));
//         render_pass.set_index_buffer(
//             render_phase.index_buffer.slice(..),
//             wgpu::IndexFormat::Uint16,
//         );
//         render_pass.draw_indexed(0..(INDICES.len() as u32), 0, 0..1);

//         drop(render_pass);

//         let command = encoder.finish();

//         queue.0.submit([command]);

//         surface_texture.present();

//         // If there are multiple primaries just use the first... TODO later problem properly
//         // making all of this work lol
//         break;
//     }
// }

impl PathtracerPhase {
    pub fn new(
        device: &wgpu::Device,
        pathtracer_output: &PathtracerOutput,
        scene_bindings: &SceneBindings,
        pathtracer_state: &PathtracerState,
        camera: &Camera,
    ) -> Self {
        let sample_shader =
            device.create_shader_module(include_spirv!(concat!(env!("OUT_DIR"), "/sample.spv")));

        let ray_extend_shader = device
            .create_shader_module(include_spirv!(concat!(env!("OUT_DIR"), "/ray_extend.spv")));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pathtracer Pipeline Layout"),
            bind_group_layouts: &[
                scene_bindings.bind_group_layout.as_ref().unwrap(),
                &pathtracer_state.bind_group_layout,
                &camera.bind_group_layout,
                &pathtracer_output.source_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let sample_main_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Pathtracer Sample Main Pipeline"),
                layout: Some(&pipeline_layout),
                module: &sample_shader,
                entry_point: Some("sampleMain"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &[],
                    zero_initialize_workgroup_memory: false,
                },
                cache: None,
            });

        let sample_cleanup_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Pathtracer Sample Cleanup Pipeline"),
                layout: Some(&pipeline_layout),
                module: &sample_shader,
                entry_point: Some("sampleCleanup"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &[],
                    zero_initialize_workgroup_memory: false,
                },
                cache: None,
            });

        let ray_extend_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Pathtracer Ray Extend Pipeline"),
                layout: Some(&pipeline_layout),
                module: &ray_extend_shader,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &[],
                    zero_initialize_workgroup_memory: false,
                },
                cache: None,
            });

        PathtracerPhase {
            sample_main_pipeline,
            sample_cleanup_pipeline,
            ray_extend_pipeline,
        }
    }
}
