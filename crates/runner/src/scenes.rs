use core::f32;
use std::collections::HashMap;
use std::f32::consts::PI;
use std::path::PathBuf;

use crate::blas;
use crate::blas::BLASData;
use crate::dielectric::DielectricData;
use crate::emissive::EmissiveData;
use crate::instance;
use crate::instance::Instance;
use crate::instance::Instances;
use crate::lambertian::LambertianData;
use crate::mesh;
use crate::metallic::MetallicData;
use crate::tlas;
use crate::tlas::TLASData;
use glam::Mat4;
use glam::Vec3;
use gltf::Gltf;
use itertools::Itertools;
use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;
use rand::random_bool;
use rand::random_range;
use rand::rng;

pub enum MaterialData {
    Lambertian(LambertianData),
    Metallic(MetallicData),
    Dielectric(DielectricData),
    Emissive(EmissiveData),
}

impl Into<MaterialData> for LambertianData {
    fn into(self) -> MaterialData {
        MaterialData::Lambertian(self)
    }
}

impl Into<MaterialData> for MetallicData {
    fn into(self) -> MaterialData {
        MaterialData::Metallic(self)
    }
}

impl Into<MaterialData> for DielectricData {
    fn into(self) -> MaterialData {
        MaterialData::Dielectric(self)
    }
}

impl Into<MaterialData> for EmissiveData {
    fn into(self) -> MaterialData {
        MaterialData::Emissive(self)
    }
}

#[derive(Default)]
pub struct SceneBuilder {
    lambertian_data: Vec<LambertianData>,
    metallic_data: Vec<MetallicData>,
    dielectric_data: Vec<DielectricData>,
    emissive_data: Vec<EmissiveData>,
    meshes: Vec<mesh::Mesh>,
    instances: Vec<Instance>,
    mesh_labels: HashMap<String, usize>,
}

pub struct Scene {
    pub lambertian_data: Vec<LambertianData>,
    pub metallic_data: Vec<MetallicData>,
    pub dielectric_data: Vec<DielectricData>,
    pub emissive_data: Vec<EmissiveData>,
    pub light_samples: Vec<u32>, // All tlas indexes which are for light sampling
    pub instances: Instances,
    pub blas_data: BLASData,
    pub tlas_data: TLASData,
}

impl SceneBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_obj(&mut self, path: &str) -> usize {
        *self.mesh_labels.entry(path.to_owned()).or_insert_with(|| {
            let mut load_options = tobj::GPU_LOAD_OPTIONS;
            load_options.single_index = false;
            let (models, _materials) = tobj::load_obj(path, &load_options).unwrap();
            self.meshes.push(mesh::Mesh::from_model(&models[0].mesh));
            self.meshes.len() - 1
        })
    }

    pub fn add_mesh(&mut self, mesh: mesh::Mesh, label: Option<&str>) -> usize {
        if let Some(label) = label {
            *self.mesh_labels.entry(label.to_owned()).or_insert_with(|| {
                self.meshes.push(mesh);
                self.meshes.len() - 1
            })
        } else {
            self.meshes.push(mesh);
            self.meshes.len() - 1
        }
    }

    pub fn get_mesh(&self, label: String) -> Option<usize> {
        self.mesh_labels.get(&label).map(|i| *i)
    }

    pub fn add_instance(&mut self, instance: instance::Instance) -> usize {
        self.instances.push(instance);
        self.instances.len() - 1
    }

    pub fn add_material(&mut self, material: impl Into<MaterialData>) -> usize {
        match material.into() {
            MaterialData::Lambertian(lambertian_data) => {
                self.lambertian_data.push(lambertian_data);
                self.lambertian_data.len() - 1
            }
            MaterialData::Metallic(metallic_data) => {
                self.metallic_data.push(metallic_data);
                self.metallic_data.len() - 1
            }
            MaterialData::Dielectric(dielectric_data) => {
                self.dielectric_data.push(dielectric_data);
                self.dielectric_data.len() - 1
            }
            MaterialData::Emissive(emissive_data) => {
                self.emissive_data.push(emissive_data);
                self.emissive_data.len() - 1
            }
        }
    }

    // pub fn load_gltf_node(&mut self, node: gltf::Node<'_>, transform: Option<Mat4>) {
    //     let mat = transform.map(|transform| {
    //         transform.mul_mat4(&glam::Mat4::from_cols_array_2d(&node.transform().matrix()))
    //     });

    //     for child in node.children() {
    //         self.load_gltf_node(child, mat);
    //     }

    //     let Some(mesh) = node.mesh() else {
    //         return;
    //     };
    // }

    // pub fn load_gltf(&mut self, path: &str) {
    //     let (gltf, buffers, _) = gltf::import(path).unwrap();

    //     let base_colour = self.add_material(LambertianData {
    //         albedo: [0.9, 0.9, 0.9, 0.0],
    //     }) as u32;

    //     let light = self.add_material(EmissiveData {
    //         albedo: [0.8, 0.8, 1.0, 0.0],
    //     }) as u32;

    //     for node in gltf.nodes() {
    //         self.load_gltf_node(node, None);

    //         for (pi, p) in m.primitives().enumerate() {
    //             let r = p.reader(|buffer| Some(&buffers[buffer.index()]));
    //             let name = m.name().map(|s| format!("{}.{}", s, pi));

    //             println!("{:?}", name);
    //             let mesh = mesh::Mesh::new(
    //                 r.read_positions().unwrap().collect(),
    //                 r.read_indices().unwrap().into_u32().collect(),
    //                 r.read_normals().unwrap().collect(),
    //             );
    //             let mesh_id = self.add_mesh(mesh, name.as_ref().map(|s| s.as_str()));

    //             let mut material = 1;

    //             let colour = p.material().pbr_metallic_roughness().base_color_factor();
    //             dbg!(colour);
    //             let mut material_idx = self.add_material(LambertianData {
    //                 albedo: [colour[0], colour[1], colour[2], 0.0],
    //             }) as u32;

    //             if p.material().emissive_factor().iter().copied().sum::<f32>() > 0.0f32 {
    //                 material = 4;
    //                 material_idx = light;
    //             }

    //             self.add_instance(Instance {
    //                 transform: instance::Transform {
    //                     scale: Vec3::splat(1.0),
    //                     rotation: Vec3::ZERO,
    //                     translation: Vec3::ZERO,
    //                     ..Default::default()
    //                 },
    //                 mesh: mesh_id as u32,
    //                 material,
    //                 material_idx,
    //                 ..Default::default()
    //             });
    //         }
    //     }
    // }

    pub fn build(mut self, device: &wgpu::Device) -> Scene {
        if self.instances.len() == 0 {
            self.instances.push(Default::default());
        }
        if self.lambertian_data.len() == 0 {
            self.lambertian_data.push(Default::default());
        }
        if self.metallic_data.len() == 0 {
            self.metallic_data.push(Default::default());
        }
        if self.dielectric_data.len() == 0 {
            self.dielectric_data.push(Default::default());
        }
        if self.emissive_data.len() == 0 {
            self.emissive_data.push(Default::default());
        }
        if self.meshes.len() == 0 {
            self.meshes.push(mesh::Mesh::cube());
        }

        dbg!(&self.instances);

        let light_samples = self
            .instances
            .iter()
            .enumerate()
            .filter(|(i, inst)| inst.material == 4)
            .map(|(i, inst)| i as u32)
            .collect_vec();

        dbg!(&light_samples);

        let instances = Instances::new(device, self.instances);

        // Make the BLAS & TLAS
        let blases = self
            .meshes
            .into_iter()
            .map(|m| blas::BLAS::new(m))
            .collect_vec();
        let tlas = tlas::TLAS::new(&blases, &instances.instances);

        let blas_data = blas::BLASData::new(device, blases);
        let tlas_data = TLASData::new(device, tlas);

        Scene {
            lambertian_data: self.lambertian_data,
            metallic_data: self.metallic_data,
            dielectric_data: self.dielectric_data,
            emissive_data: self.emissive_data,
            instances,
            blas_data,
            tlas_data,
            light_samples,
        }
    }
}

pub fn sponza_scene(sb: &mut SceneBuilder) {
    // sb.load_gltf("assets/main_sponza/NewSponza_Curtains_glTF.gltf");
    // // sb.load_gltf("assets/sponza/Sponza.gltf");
    // let lc = sb.add_mesh(mesh::Mesh::cube(), Some("LightCube")) as u32;
    // let lcm = sb.add_material(EmissiveData {
    //     albedo: [0.7, 0.8, 1.0, 0.0],
    // }) as u32;
    // sb.add_instance(Instance {
    //     transform: instance::Transform {
    //         scale: Vec3::splat(2.0),
    //         rotation: Vec3::splat(0.0),
    //         translation: Vec3::new(12.617, 4.52, -0.23),
    //         ..Default::default()
    //     },
    //     mesh: lc,
    //     material: 4,
    //     material_idx: lcm,
    //     ..Default::default()
    // });
}

pub fn boxes_scene(scene_builder: &mut SceneBuilder) {
    scene_builder.add_obj("assets/suzanne.obj");
    scene_builder.add_obj("assets/teapot.obj");
    scene_builder.add_obj("assets/dragon.obj");
    let quad_id = scene_builder.add_mesh(mesh::Mesh::rect(), Some("unit_rect")) as u32;
    let cube_id = scene_builder.add_mesh(mesh::Mesh::cube(), Some("unit_cube")) as u32;

    // Make material data for lambertian:
    // Basic gray;
    let base_colour = scene_builder.add_material(LambertianData {
        albedo: [0.73, 0.73, 0.73, 0.0],
    }) as u32;

    // Fun colours:
    let red = scene_builder.add_material(LambertianData {
        albedo: [0.65, 0.05, 0.05, 0.0],
    }) as u32;

    let green = scene_builder.add_material(LambertianData {
        albedo: [0.12, 0.45, 0.15, 0.0],
    }) as u32;

    let blue = scene_builder.add_material(LambertianData {
        albedo: [0.05, 0.10, 0.60, 0.0],
    }) as u32;

    let purple = scene_builder.add_material(LambertianData {
        albedo: [0.35, 0.08, 0.45, 0.0],
    }) as u32;

    let gold = scene_builder.add_material(MetallicData {
        albedo: [0.0, 0.83, 1.0, 0.0],
        fuzz: 0.2,
        ..Default::default()
    }) as u32;
    let mirror = scene_builder.add_material(MetallicData {
        albedo: [1.0, 1.0, 1.0, 0.0],
        fuzz: 0.01,
        ..Default::default()
    }) as u32;

    let glass = scene_builder.add_material(DielectricData {
        albedo: [1.0, 1.0, 1.0, 0.0],
        ir: 1.2,
        ..Default::default()
    }) as u32;

    // let light = scene_builder.add_material(EmissiveData {
    //     albedo: [0.5, 0.8, 0.9, 1.0].map(|i| i * 800.0),
    // }) as u32;
    let light = scene_builder.add_material(EmissiveData {
        albedo: [1.0, 1.0, 1.0, 1.0].map(|i| i * 800.0),
    }) as u32;

    let half = 5.0;
    let depth = 10.0;
    let z_mid = depth * 0.5;
    let offset = Vec3::new(0.0, 0.0, half);

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
            material_idx: base_colour,
            ..Default::default()
        },
        // Front wall:
        // Instance {
        //     transform: instance::Transform {
        //         scale: Vec3::new(half * 2.0, half * 2.0, 1.0),
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
            material_idx: base_colour,
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
            material_idx: base_colour,
            ..Default::default()
        },
        // Ceiling Light
        Instance {
            transform: instance::Transform {
                scale: Vec3::new(1.0, 0.5, 6.0),
                rotation: Vec3::new(0.0, 0.0, 0.0),
                translation: Vec3::new(-half + 1.0, half - 0.25, half),
                ..Default::default()
            },
            mesh: cube_id,
            material: 4,
            material_idx: light,
            ..Default::default()
        },
        Instance {
            transform: instance::Transform {
                scale: Vec3::new(1.0, 0.5, 6.0),
                rotation: Vec3::new(0.0, 0.0, 0.0),
                translation: Vec3::new(half - 1.0, half - 0.25, half),
                ..Default::default()
            },
            mesh: cube_id,
            material: 4,
            material_idx: light,
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
            material_idx: red,
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
            material_idx: green,
            ..Default::default()
        },
        // Cube:
        Instance {
            transform: instance::Transform {
                scale: Vec3::new(2.5, 6.0, 2.5),
                rotation: Vec3::new(0.0, PI * -0.4, 0.0),
                translation: Vec3::new(-1.0, -half + 3.0, half + 2.0),
                ..Default::default()
            },
            mesh: cube_id,
            material: 2,
            material_idx: mirror,
            ..Default::default()
        },
        Instance {
            transform: instance::Transform {
                scale: Vec3::new(2.5, 2.99, 2.5),
                rotation: Vec3::new(0.0, PI * -0.1, 0.0),
                translation: Vec3::new(0.4, -half + 1.5, half - 1.8),
                ..Default::default()
            },
            mesh: cube_id,
            material: 1, // Dielectric
            material_idx: blue,
            ..Default::default()
        },
    ]
    .into_iter()
    .for_each(|mut i| {
        i.transform.translation += offset;
        scene_builder.add_instance(i);
    });
}

pub(crate) fn grid_scene(scene_builder: &mut SceneBuilder) {
    use std::f32::consts::PI;

    let suzanne_id = scene_builder.add_obj("assets/suzanne.obj") as u32;
    let teapot_id = scene_builder.add_obj("assets/teapot.obj") as u32;
    let dragon_id = scene_builder.add_obj("assets/dragon.obj") as u32;
    let cube_id = scene_builder.add_mesh(mesh::Mesh::cube(), Some("unit_cube")) as u32;

    let mesh_ids = [suzanne_id, teapot_id, cube_id];

    let mut lambertian_ids: Vec<u32> = vec![];

    for _ in 0..15 {
        lambertian_ids.push(scene_builder.add_material(LambertianData {
            albedo: [
                random_range(0.0..=1.0),
                random_range(0.0..=1.0),
                random_range(0.0..=1.0),
                0.0,
            ],
        }) as u32);
    }

    let mut metallic_ids: Vec<u32> = Vec::with_capacity(lambertian_ids.len());
    for _ in 0..lambertian_ids.len() {
        metallic_ids.push(scene_builder.add_material(MetallicData {
            albedo: [
                random_range(0.0..=1.0),
                random_range(0.0..=1.0),
                random_range(0.0..=1.0),
                0.0,
            ],
            fuzz: random_range(-1.0..=1.0f32).clamp(0.0, 1.0),
            ..Default::default()
        }) as u32);
    }

    let mut emissive_ids: Vec<u32> = Vec::with_capacity(lambertian_ids.len());
    for _ in 0..lambertian_ids.len() {
        emissive_ids.push(
            scene_builder.add_material(EmissiveData {
                albedo: [
                    random_range(0.4..=1.0),
                    random_range(0.4..=1.0),
                    random_range(0.4..=1.0),
                    1.0,
                ]
                .map(|i| i * 800.0),
                ..Default::default()
            }) as u32,
        );
    }

    let mut dielectric_ids: Vec<u32> = Vec::with_capacity(lambertian_ids.len());
    for _ in 0..lambertian_ids.len() {
        dielectric_ids.push(scene_builder.add_material(DielectricData {
            albedo: [
                random_range(0.0..=1.0),
                random_range(0.0..=1.0),
                random_range(0.0..=1.0),
                0.0,
            ],
            ir: random_range(1.0..=1.8f32),
            ..Default::default()
        }) as u32);
    }

    let mut rng = rng();
    let weights = [2.0, 1.0, 1.0, 0.1];
    let mut dist = WeightedIndex::new(&weights).unwrap();
    for x in 1..=10 {
        for y in 0..5 {
            for z in 1..=10 {
                let material = (dist.sample(&mut rng) as u32) + 1;

                let material_idx = match material {
                    1 => lambertian_ids[random_range(0..lambertian_ids.len() as u32) as usize],
                    2 => metallic_ids[random_range(0..metallic_ids.len() as u32) as usize],
                    3 => dielectric_ids[random_range(0..dielectric_ids.len() as u32) as usize],
                    4 => emissive_ids[random_range(0..emissive_ids.len() as u32) as usize],
                    _ => unreachable!(),
                };

                let mesh = mesh_ids[random_range(0..mesh_ids.len() as u32) as usize];

                let scale = Vec3::splat(random_range(0.5..=1.25));
                let rotation = Vec3::ZERO.map(|_| random_range(0.0..=1.0 * PI)); // match your old behavior

                let base_translation = Vec3::new(x as f32 * 2.0, y as f32 * 2.0, z as f32 * 2.0);
                let jitter = Vec3::new(
                    random_range(-0.25..=0.25),
                    random_range(-0.25..=0.25),
                    random_range(-0.25..=0.25),
                );

                scene_builder.add_instance(Instance {
                    transform: instance::Transform {
                        scale,
                        rotation,
                        translation: base_translation + jitter,
                        ..Default::default()
                    },
                    mesh,
                    material,
                    material_idx,
                    ..Default::default()
                });
            }
        }
    }
}

pub(crate) fn cornell_scene(scene_builder: &mut SceneBuilder) {
    let suzanne_id = scene_builder.add_obj("assets/suzanne.obj") as u32;
    let teapot_id = scene_builder.add_obj("assets/teapot.obj") as u32;
    let dragon_id = scene_builder.add_obj("assets/dragon.obj") as u32;
    let quad_id = scene_builder.add_mesh(mesh::Mesh::rect(), Some("unit_rect")) as u32;
    let cube_id = scene_builder.add_mesh(mesh::Mesh::cube(), Some("unit_cube")) as u32;

    // Make material data for lambertian:
    // Basic gray;
    let base_colour = scene_builder.add_material(LambertianData {
        albedo: [0.73, 0.73, 0.73, 0.0],
    }) as u32;

    // Fun colours:
    let red = scene_builder.add_material(LambertianData {
        albedo: [0.65, 0.05, 0.05, 0.0],
    }) as u32;

    let green = scene_builder.add_material(LambertianData {
        albedo: [0.12, 0.45, 0.15, 0.0],
    }) as u32;

    let blue = scene_builder.add_material(LambertianData {
        albedo: [0.05, 0.10, 0.60, 0.0],
    }) as u32;

    let purple = scene_builder.add_material(LambertianData {
        albedo: [0.35, 0.08, 0.45, 0.0],
    }) as u32;

    let gold = scene_builder.add_material(MetallicData {
        albedo: [0.0, 0.83, 1.0, 0.0],
        fuzz: 0.2,
        ..Default::default()
    }) as u32;
    let mirror = scene_builder.add_material(MetallicData {
        albedo: [1.0, 1.0, 1.0, 0.0],
        fuzz: 0.0,
        ..Default::default()
    }) as u32;

    let glass = scene_builder.add_material(DielectricData {
        albedo: [1.0, 1.0, 1.0, 0.0],
        ir: 1.2,
        ..Default::default()
    }) as u32;

    // let light = scene_builder.add_material(EmissiveData {
    //     albedo: [0.5, 0.8, 0.9, 1.0].map(|i| i + 0.0),
    // }) as u32;
    let light = scene_builder.add_material(EmissiveData {
        albedo: [0.5, 0.8, 0.9, 1.0].map(|i| i * 800.0),
    }) as u32;

    let half = 5.0;
    let depth = 10.0;
    let z_mid = depth * 0.5;
    let offset = Vec3::new(0.0, 0.0, half);

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
            material_idx: base_colour,
            ..Default::default()
        },
        // Front wall:
        // Instance {
        //     transform: instance::Transform {
        //         scale: Vec3::new(half * 2.0, half * 2.0, 1.0),
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
            material_idx: base_colour,
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
            material_idx: base_colour,
            ..Default::default()
        },
        // Ceiling Light
        Instance {
            transform: instance::Transform {
                scale: Vec3::new(1.0, 0.5, 6.0),
                rotation: Vec3::new(0.0, 0.0, 0.0),
                translation: Vec3::new(-half + 1.0, half - 0.25, half),
                ..Default::default()
            },
            mesh: cube_id,
            material: 4,
            material_idx: light,
            ..Default::default()
        },
        Instance {
            transform: instance::Transform {
                scale: Vec3::new(1.0, 0.5, 6.0),
                rotation: Vec3::new(0.0, 0.0, 0.0),
                translation: Vec3::new(half - 1.0, half - 0.25, half),
                ..Default::default()
            },
            mesh: cube_id,
            material: 4,
            material_idx: light,
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
            material_idx: red,
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
            material_idx: green,
            ..Default::default()
        },
        // Dragon
        Instance {
            transform: instance::Transform {
                scale: Vec3::splat(6.0),
                rotation: Vec3::new(0.0, PI * 0.25, 0.0),
                translation: Vec3::new(0.0, -half + 1.7, half),
                ..Default::default()
            },
            mesh: dragon_id,
            material: 2,
            material_idx: gold,
            ..Default::default()
        },
    ]
    .into_iter()
    .for_each(|mut i| {
        i.transform.translation += offset;
        scene_builder.add_instance(i);
    });

    // // Dragon Centered:
    // // Suzanne Centered:
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
    // // Teapot Centered:
    // // Instance {
    // //     transform: instance::Transform {
    // //         scale: Vec3::splat(4.0),
    // //         rotation: Vec3::new(0.0, 0.0, 0.0),
    // //         translation: Vec3::new(0.0, 0.0, half),
    // //         ..Default::default()
    // //     },
    // //     mesh: 1,
    // //     material: 3,
    // //     material_idx: 0,
    // //     ..Default::default()
    // // },
}

pub(crate) fn windows_scene(sb: &mut SceneBuilder) {
    let quad_id = sb.add_mesh(mesh::Mesh::rect(), Some("unit_rect")) as u32;
    let cube_id = sb.add_mesh(mesh::Mesh::cube(), Some("unit_cube")) as u32;

    // 0 = nice neutral gray
    let wall_lambert = sb.add_material(LambertianData {
        albedo: [0.75, 0.75, 0.78, 0.0],
    }) as u32;

    let _default_metal = sb.add_material(MetallicData {
        ..Default::default()
    }) as u32;
    let skylight = sb.add_material(EmissiveData {
        albedo: [1.0, 1.0, 1.0, 1.0].map(|x| x * 200.0),
    }) as u32;

    let dielectric_ids: Vec<u32> = (0..100)
        .map(|_i| {
            sb.add_material(DielectricData {
                albedo: [0, 0, 0, 0].map(|_| random_range(0.0..=1.0)),
                ir: 1.2,
                ..Default::default()
            }) as u32
        })
        .collect();

    let half = 5.0;
    let depth = 500.0;
    let z_mid = depth * 0.5;
    let offset = Vec3::new(0.0, 0.0, half);

    let base_walls = vec![
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
            material_idx: wall_lambert,
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
            material_idx: wall_lambert,
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
            material_idx: wall_lambert,
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
            material_idx: wall_lambert,
            ..Default::default()
        },
        // The skylight
        Instance {
            transform: instance::Transform {
                scale: Vec3::new(500000.0, 1.0, 500000.0),
                rotation: Vec3::new(0.0, 0.0, 0.0),
                translation: Vec3::new(0.0, 10000.0, 0.0),
                ..Default::default()
            },
            mesh: cube_id,
            material: 4,
            material_idx: skylight,
            ..Default::default()
        },
    ];

    base_walls
        .into_iter()
        .map(|mut i| {
            i.transform.translation += offset;
            i
        })
        .for_each(|i| {
            sb.add_instance(i);
        });

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
        sb.add_instance(Instance {
            transform: instance::Transform {
                scale: Vec3::new(strip_w, half * 2.0, 1.0),
                rotation: Vec3::new(0.0, PI * 0.5, 0.0),
                translation: Vec3::new(half, 0.0, strip_center_z) + offset,
                ..Default::default()
            },
            mesh: cube_id,
            material: 1,
            material_idx: wall_lambert,
            ..Default::default()
        });

        z_cursor += strip_w;

        // Window segment
        let win_center_z = z_cursor + window_w * 0.5;
        let diel_idx = dielectric_ids[i % dielectric_ids.len()];

        sb.add_instance(Instance {
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
        sb.add_instance(Instance {
            transform: instance::Transform {
                scale: Vec3::new(remaining, half * 2.0, 1.0),
                rotation: Vec3::new(0.0, PI * 0.5, 0.0),
                translation: Vec3::new(half, 0.0, strip_center_z) + offset,
                ..Default::default()
            },
            mesh: cube_id,
            material: 1,
            material_idx: wall_lambert,
            ..Default::default()
        });
    }

    // Bottom/top window gaps:
    sb.add_instance(Instance {
        transform: instance::Transform {
            scale: Vec3::new(depth, half - window_h / 2.0, 1.0),
            rotation: Vec3::new(0.0, PI * 0.5, 0.0),
            translation: Vec3::new(half, window_h / 4.0 + half / 2.0, z_mid) + offset,
            ..Default::default()
        },
        mesh: cube_id,
        material: 1,
        material_idx: wall_lambert,
        ..Default::default()
    });

    sb.add_instance(Instance {
        transform: instance::Transform {
            scale: Vec3::new(depth, half - window_h / 2.0, 1.0),
            rotation: Vec3::new(0.0, PI * 0.5, 0.0),
            translation: Vec3::new(half, -window_h / 4.0 - half / 2.0, z_mid) + offset,
            ..Default::default()
        },
        mesh: cube_id,
        material: 1,
        material_idx: wall_lambert,
        ..Default::default()
    });
}
