use core::f32;
use std::f32::consts::PI;

use crate::blas;
use crate::blas::BLASData;
use crate::dielectric::DielectricData;
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
    Vec<DielectricData>,
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

    let dielectric_data = vec![DielectricData {
        ..Default::default()
    }];

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
        dielectric_data,
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
    Vec<DielectricData>,
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
            albedo: [0.9, 0.9, 0.9, 0.0],
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

    let dielectric_data = vec![DielectricData {
        albedo: [0.83, 1.0, 0.0, 0.0],
        ir: 1.47,
        ..Default::default()
    }];

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
            // Instance {
            //     transform: instance::Transform {
            //         scale: Vec3::new(half * 2.0, depth, 1.0),
            //         rotation: Vec3::new(-PI * 0.5, 0.0, 0.0),
            //         translation: Vec3::new(0.0, half, z_mid),
            //         ..Default::default()
            //     },
            //     mesh: quad_id,
            //     material: 1,
            //     material_idx: 0,
            //     ..Default::default()
            // },
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
            // Dragon Centered:
            Instance {
                transform: instance::Transform {
                    scale: Vec3::splat(6.0),
                    rotation: Vec3::new(0.0, PI * 0.25, 0.0),
                    translation: Vec3::new(0.0, -half + 1.7, half),
                    ..Default::default()
                },
                mesh: 2,
                material: 3,
                material_idx: 0,
                ..Default::default()
            },
            // Suzanne Centered:
            // Instance {
            //     transform: instance::Transform {
            //         scale: Vec3::splat(4.0),
            //         rotation: Vec3::new(0.0, 0.0, 0.0),
            //         translation: Vec3::new(0.0, 0.0, half),
            //         ..Default::default()
            //     },
            //     mesh: 0,
            //     material: 3,
            //     material_idx: 0,
            //     ..Default::default()
            // },
            // Teapot Centered:
            // Instance {
            //     transform: instance::Transform {
            //         scale: Vec3::splat(4.0),
            //         rotation: Vec3::new(0.0, 0.0, 0.0),
            //         translation: Vec3::new(0.0, 0.0, half),
            //         ..Default::default()
            //     },
            //     mesh: 1,
            //     material: 3,
            //     material_idx: 0,
            //     ..Default::default()
            // },
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
        dielectric_data,
        instances,
        blas_data,
        tlas_data,
    )
}

pub(crate) fn windows(
    device: &wgpu::Device,
) -> (
    Vec<LambertianData>,
    Vec<MetallicData>,
    Vec<DielectricData>,
    Instances,
    BLASData,
    TLASData,
) {
    let mut load_options = tobj::GPU_LOAD_OPTIONS;
    load_options.single_index = false;

    let mut meshes = Vec::new();

    meshes.push(mesh::Mesh::rect());
    let quad_id = (meshes.len() - 1) as u32;
    meshes.push(mesh::Mesh::cube());
    let cube_id = (meshes.len() - 1) as u32;

    // 0 = nice neutral gray
    let lambertian_data = vec![LambertianData {
        albedo: [0.75, 0.75, 0.78, 0.0],
    }];

    let metallic_data = vec![MetallicData {
        ..Default::default()
    }];

    // let dielectric_data = vec![
    //     DielectricData {
    //         albedo: [0.65, 1.00, 0.20, 0.0],
    //         ir: 1.3,
    //         ..Default::default()
    //     },
    //     DielectricData {
    //         albedo: [1.00, 0.60, 0.20, 0.0],
    //         ir: 1.3,
    //         ..Default::default()
    //     },
    //     DielectricData {
    //         albedo: [0.35, 0.35, 1.00, 0.0],
    //         ir: 1.3,
    //         ..Default::default()
    //     },
    //     DielectricData {
    //         albedo: [0.85, 0.20, 1.00, 0.0],
    //         ir: 1.3,
    //         ..Default::default()
    //     },
    //     DielectricData {
    //         albedo: [0.45, 1.00, 0.75, 0.0],
    //         ir: 1.3,
    //         ..Default::default()
    //     },
    // ];

    let dielectric_data = (0..100)
        .into_iter()
        .map(|i| DielectricData {
            albedo: [0, 0, 0, 0].map(|a| random_range(0.0..=1.0)),
            ir: 1.2,
            ..Default::default()
        })
        .collect_vec();

    let mut instances = vec![];

    let half = 5.0;
    let depth = 500.0;
    let z_mid = depth * 0.5;
    let offset = Vec3::new(0.0, 0.0, half);

    instances.extend(
        vec![
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
            // Ceiling:
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
            // Left wall:
            Instance {
                transform: instance::Transform {
                    scale: Vec3::new(depth, half * 2.0, 1.0),
                    rotation: Vec3::new(0.0, -PI * 0.5, 0.0),
                    translation: Vec3::new(-half, 0.0, z_mid),
                    ..Default::default()
                },
                mesh: quad_id,
                material: 1,
                material_idx: 0,
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

    // Right wall as: [thin strip][window][thin strip][window]...[thin strip end]
    let window_count: usize = 100;
    let strip_w = 0.01; // thin bits of wall between windows
    let window_w = 8.0; // width along corridor (z-direction) per window
    let window_h = 8.0; // tall window
    let y_center = 0.0; // centered vertically
    let z_start = 1.0; // start a little into the corridor

    let mut z_cursor = z_start;
    for i in 0..window_count {
        // Strip segment
        let strip_center_z = z_cursor + strip_w * 0.5;
        instances.push(Instance {
            transform: instance::Transform {
                scale: Vec3::new(strip_w, half * 2.0, 1.0),
                rotation: Vec3::new(0.0, PI * 0.5, 0.0),
                translation: Vec3::new(half, 0.0, strip_center_z) + offset,
                ..Default::default()
            },
            mesh: cube_id,
            material: 1,
            material_idx: 0,
            ..Default::default()
        });

        z_cursor += strip_w;

        // Window segment
        let win_center_z = z_cursor + window_w * 0.5;
        let diel_idx = (i % dielectric_data.len()) as u32;

        instances.push(Instance {
            transform: instance::Transform {
                scale: Vec3::new(window_w, window_h, 1.0),
                rotation: Vec3::new(0.0, PI * 0.5, 0.0),
                translation: Vec3::new(half, y_center, win_center_z) + offset,
                ..Default::default()
            },
            mesh: cube_id,
            material: 3,
            material_idx: diel_idx,
            ..Default::default()
        });

        z_cursor += window_w;
    }

    // Final strip to fill in remainder:
    if z_cursor < depth {
        let remaining = depth - z_cursor;
        let strip_center_z = z_cursor + remaining * 0.5;
        instances.push(Instance {
            transform: instance::Transform {
                scale: Vec3::new(remaining, half * 2.0, 1.0),
                rotation: Vec3::new(0.0, PI * 0.5, 0.0),
                translation: Vec3::new(half, 0.0, strip_center_z) + offset,
                ..Default::default()
            },
            mesh: cube_id,
            material: 1,
            material_idx: 0,
            ..Default::default()
        });
    }

    // Bottom/top window gaps:
    instances.extend(vec![
        Instance {
            transform: instance::Transform {
                scale: Vec3::new(depth, half - window_h / 2.0, 1.0),
                rotation: Vec3::new(0.0, PI * 0.5, 0.0),
                translation: Vec3::new(half, window_h / 4.0 + half / 2.0, z_mid),
                ..Default::default()
            },
            mesh: cube_id,
            material: 1,
            material_idx: 0,
            ..Default::default()
        },
        Instance {
            transform: instance::Transform {
                scale: Vec3::new(depth, half - window_h / 2.0, 1.0),
                rotation: Vec3::new(0.0, PI * 0.5, 0.0),
                translation: Vec3::new(half, -window_h / 4.0 - half / 2.0, z_mid),
                ..Default::default()
            },
            mesh: cube_id,
            material: 1,
            material_idx: 0,
            ..Default::default()
        },
    ]);

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
        dielectric_data,
        instances,
        blas_data,
        tlas_data,
    )
}
