use core::f32;
use std::f32::consts::PI;

use crate::blas;
use crate::blas::BLASData;
use crate::instance;
use crate::instance::Instance;
use crate::instance::Instances;
use crate::lambertian::LambertianData;
use crate::mesh;
use crate::metallic::MetallicData;
use crate::tlas;
use crate::tlas::TLASData;
use glam::Vec3;
use itertools::Itertools;
use rand::random_range;

pub(crate) fn grid_scene(
    device: &wgpu::Device,
) -> (
    Vec<LambertianData>,
    Vec<MetallicData>,
    Instances,
    BLASData,
    TLASData,
) {
    // Load models:
    let mut load_options = tobj::GPU_LOAD_OPTIONS;
    load_options.single_index = false;

    let mut meshes = Vec::new();

    // Suzanne!
    let (models, materials) = tobj::load_obj("assets/suzanne.obj", &load_options).unwrap();
    meshes.push(mesh::Mesh::from_model(&models[0].mesh));

    // Teapot!
    let (models, materials) = tobj::load_obj("assets/teapot.obj", &load_options).unwrap();
    meshes.push(mesh::Mesh::from_model(&models[0].mesh));

    // Teapot!
    let (models, materials) = tobj::load_obj("assets/dragon.obj", &load_options).unwrap();
    meshes.push(mesh::Mesh::from_model(&models[0].mesh));

    // Make material data for lambertian:
    let mut lambertian_data = vec![
        // For now, 3 instances, r/g/b each
        LambertianData {
            albedo: [0.9, 0.9, 0.9, 0.0],
        },
        LambertianData {
            albedo: [0.8, 0.8, 0.9, 0.0],
        },
        LambertianData {
            albedo: [0.8, 0.9, 0.8, 0.0],
        },
        LambertianData {
            albedo: [0.9, 0.8, 0.8, 0.0],
        },
    ];
    for _ in 0..10 {
        lambertian_data.push(LambertianData {
            albedo: [0.0, 0.0, 0.0, 0.0].map(|_| random_range(0.0..=1.0)),
        });
    }

    // Make material data for metallics:
    let metallic_data = lambertian_data
        .clone()
        .into_iter()
        .map(|ld| MetallicData {
            albedo: ld.albedo.map(|_| random_range(0.0..=1.0)),
            fuzz: random_range(-1.0..=1.0f32).clamp(0.0, 1.0),
            ..Default::default()
        })
        .collect_vec();

    // Instances:
    let mut instances = vec![];
    for x in 1..=10 {
        for y in 0..5 {
            for z in 1..=10 {
                let material = random_range(1..=2);
                let material_idx = match material {
                    1 => random_range(0..lambertian_data.len() as u32),
                    2 => random_range(0..metallic_data.len() as u32),
                    _ => panic!(),
                };
                instances.push(Instance {
                    transform: instance::Transform {
                        scale: Vec3::splat(random_range(0.5..=1.25)),
                        rotation: Vec3::ZERO.map(|_| random_range(0.0..=1.0 * f32::consts::PI)),
                        translation: Vec3::new(x as f32 * 2.0, y as f32 * 2.0, z as f32 * 2.0)
                            .map(|i| i + random_range(-0.25..=0.25)),
                        ..Default::default()
                    },
                    mesh: random_range(0..meshes.len() as u32),
                    material: material,
                    material_idx: material_idx,
                    ..Default::default()
                });
            }
        }
    }

    let instances = Instances::new(device, instances);

    // Make the BLAS & TLAS
    let blases = meshes.into_iter().map(|m| blas::BLAS::new(m)).collect_vec();
    dbg!(blases.iter().map(|blas| blas.nodes[0]).collect_vec());
    let tlas = tlas::TLAS::new(&blases, &instances.instances);

    let blas_data = blas::BLASData::new(device, blases);
    let tlas_data = TLASData::new(device, tlas);
    (
        lambertian_data,
        metallic_data,
        instances,
        blas_data,
        tlas_data,
    )
}

pub(crate) fn cornell_scene(
    device: &wgpu::Device,
) -> (
    Vec<LambertianData>,
    Vec<MetallicData>,
    Instances,
    BLASData,
    TLASData,
) {
    // Load models:
    let mut load_options = tobj::GPU_LOAD_OPTIONS;
    load_options.single_index = false;

    let mut meshes = Vec::new();

    // Suzanne!
    let (models, materials) = tobj::load_obj("assets/suzanne.obj", &load_options).unwrap();
    meshes.push(mesh::Mesh::from_model(&models[0].mesh));

    // Teapot!
    let (models, materials) = tobj::load_obj("assets/teapot.obj", &load_options).unwrap();
    meshes.push(mesh::Mesh::from_model(&models[0].mesh));

    // Teapot!
    let (models, materials) = tobj::load_obj("assets/dragon.obj", &load_options).unwrap();
    meshes.push(mesh::Mesh::from_model(&models[0].mesh));

    meshes.push(mesh::Mesh::rect());
    let quad_id = (meshes.len() - 1) as u32;

    // Make material data for lambertian:
    let mut lambertian_data = vec![
        // Basic gray/white:
        LambertianData {
            albedo: [1.0, 1.0, 1.0, 0.0],
        },
        // For now, 3 instances, r/g/b each
        LambertianData {
            albedo: [0.4, 0.4, 0.9, 0.0],
        },
        LambertianData {
            albedo: [0.4, 0.9, 0.4, 0.0],
        },
        LambertianData {
            albedo: [0.9, 0.4, 0.4, 0.0],
        },
    ];
    for _ in 0..10 {
        lambertian_data.push(LambertianData {
            albedo: [0.0, 0.0, 0.0, 0.0].map(|_| random_range(0.0..=1.0)),
        });
    }

    // Make material data for metallics:
    let metallic_data = vec![
        MetallicData {
            albedo: [0.0, 0.83, 1.0, 0.0],
            fuzz: 0.2,
            ..Default::default()
        },
        MetallicData {
            albedo: [1.0, 1.0, 1.0, 0.0],
            fuzz: 0.0,
            ..Default::default()
        },
    ];

    // Instances:
    let mut instances = vec![];

    let half = 5.0;
    let depth = 10.0;
    let z_mid = depth * 0.5;
    let offset = Vec3::new(0.0, 0.0, half);

    instances.append(
        &mut vec![
            // Back wall:
            Instance {
                transform: instance::Transform {
                    scale: Vec3::new(half * 2.0, half * 2.0, 1.0),
                    rotation: Vec3::ZERO,
                    translation: Vec3::new(0.0, 0.0, depth),
                    ..Default::default()
                },
                mesh: quad_id,
                material: 1,
                material_idx: 0,
                ..Default::default()
            },
            // Front wall:
            // Instance {
            //     transform: instance::Transform {
            //         scale: Vec3::new(half * 2.0, half * 1.8, 1.0),
            //         rotation: Vec3::ZERO,
            //         translation: Vec3::new(0.0, 0.0, 0.0),
            //         ..Default::default()
            //     },
            //     mesh: quad_id,
            //     material: 1,
            //     material_idx: 0,
            //     ..Default::default()
            // },
            // Floor:
            Instance {
                transform: instance::Transform {
                    scale: Vec3::new(half * 2.0, depth, 1.0),
                    rotation: Vec3::new(PI * 0.5, 0.0, 0.0),
                    translation: Vec3::new(0.0, -half, z_mid),
                    ..Default::default()
                },
                mesh: quad_id,
                material: 1,
                material_idx: 0,
                ..Default::default()
            },
            // Ceiling
            Instance {
                transform: instance::Transform {
                    scale: Vec3::new(half * 2.0, depth, 1.0),
                    rotation: Vec3::new(-PI * 0.5, 0.0, 0.0),
                    translation: Vec3::new(0.0, half, z_mid),
                    ..Default::default()
                },
                mesh: quad_id,
                material: 1,
                material_idx: 0,
                ..Default::default()
            },
            // Left wall
            Instance {
                transform: instance::Transform {
                    scale: Vec3::new(depth, half * 2.0, 1.0),
                    rotation: Vec3::new(0.0, -PI * 0.5, 0.0), // +Z -> +X
                    translation: Vec3::new(-half, 0.0, z_mid),
                    ..Default::default()
                },
                mesh: quad_id,
                material: 1,
                material_idx: 1,
                ..Default::default()
            },
            // Right wall
            Instance {
                transform: instance::Transform {
                    scale: Vec3::new(depth, half * 2.0, 1.0),
                    rotation: Vec3::new(0.0, PI * 0.5, 0.0), // +Z -> -X
                    translation: Vec3::new(half, 0.0, z_mid),
                    ..Default::default()
                },
                mesh: quad_id,
                material: 1,
                material_idx: 2,
                ..Default::default()
            },
            Instance {
                transform: instance::Transform {
                    scale: Vec3::splat(6.0),
                    rotation: Vec3::new(0.0, PI * 0.25, 0.0),
                    translation: Vec3::new(0.0, -half + 1.7, half),
                    ..Default::default()
                },
                mesh: 2,
                material: 2,
                material_idx: 1,
                ..Default::default()
            },
        ]
        .into_iter()
        .map(|mut i| {
            i.transform.translation += offset;
            i
        })
        .collect_vec(),
    );

    let instances = Instances::new(device, instances);

    // Make the BLAS & TLAS
    let blases = meshes.into_iter().map(|m| blas::BLAS::new(m)).collect_vec();
    dbg!(blases.iter().map(|blas| blas.nodes[0]).collect_vec());
    let tlas = tlas::TLAS::new(&blases, &instances.instances);

    let blas_data = blas::BLASData::new(device, blases);
    let tlas_data = TLASData::new(device, tlas);
    (
        lambertian_data,
        metallic_data,
        instances,
        blas_data,
        tlas_data,
    )
}
