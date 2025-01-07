use crate::particle::{ParticleInstance, ParticleSystem, ParticleSystemData};
use crate::{model, resource, texture};
use cgmath::Vector3;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::ops::{Add, Mul};
use rand::random;
use crate::model::{Material, Model};
use crate::util::BoundingBox;

const BOUNDING_BOX_X: f32 = 3.0;
const BOUNDING_BOX_Y: f32 = 1.5;

pub(crate) enum ScreenSaverType {
    Snow,
}

lazy_static! {
    static ref SCREEN_SAVER_NAMES: HashMap<String, ScreenSaverType> =
        HashMap::from([("snow".to_string(), ScreenSaverType::Snow)]);
}

pub trait ScreenSaver {
    fn setup(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout);
    fn get_models(&self) -> &Vec<model::Model>;
    fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, dt: std::time::Duration);
    fn get_background_color(&self) -> wgpu::Color;
}
pub struct SnowScreenSaver {
    pub(crate) models: Vec<model::Model>,
}

impl ScreenSaver for SnowScreenSaver {
    fn setup(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) {
        //ground defined first so it gets drawn first and doesn't get occluded by the snow
        let ground1 = include_bytes!("resources/textures/ground1.png");
        let diffuse_texture =
            texture::Texture::from_bytes(&device, &queue, ground1, "ground1.png").unwrap();
        let billboard = resource::create_billboard(
            6.0,
            3.0,
            Vector3::new(0.0, 0.0, 0.1),
            diffuse_texture,
            &device,
            &layout,
        )
        .unwrap();
        self.models.push(billboard);

        let ground2 = include_bytes!("resources/textures/ground2.png");
        let diffuse_texture =
            texture::Texture::from_bytes(&device, &queue, ground2, "ground2.png").unwrap();
        let billboard = resource::create_billboard(
            6.0,
            3.0,
            Vector3::new(0.0, 0.0, 0.3),
            diffuse_texture,
            &device,
            &layout,
        )
        .unwrap();
        self.models.push(billboard);

        let ground3 = include_bytes!("resources/textures/ground3.png");
        let diffuse_texture =
            texture::Texture::from_bytes(&device, &queue, ground3, "ground3.png").unwrap();
        let billboard = resource::create_billboard(
            6.0,
            3.0,
            Vector3::new(0.0, 0.0, 0.5),
            diffuse_texture,
            &device,
            &layout,
        )
        .unwrap();
        self.models.push(billboard);

        let snow1 = include_bytes!("resources/textures/snow1.png");
        let snow2 = include_bytes!("resources/textures/snow2.png");
        let diffuse_textures = [
            texture::Texture::from_bytes(&device, &queue, snow1, "snow1.png").unwrap(),
            texture::Texture::from_bytes(&device, &queue, snow2, "snow2.png").unwrap(),
        ];
        for diffuse_texture in diffuse_textures {
            let mut snow_particle_system = ParticleSystem::create_billboard(
                0.03,
                0.03,
                Vector3::new(0.0, 0.0, 0.0),
                ParticleSystemData::new(
                    BoundingBox::new_with_size(
                        Vector3::new(0.0, 0.0, 0.5),
                        BOUNDING_BOX_X * 2.0,
                        BOUNDING_BOX_Y * 2.0,
                        1.0,
                    )
                ),
                &device
            ).unwrap();

            let mut snow_material = Material::new(
                diffuse_texture,
                &device,
                &layout,
            );

            snow_particle_system.populate_random(7500, device);

            for mut particle in snow_particle_system.instances.as_mut_slice() {
                particle.position.z = 1.0 - particle.position.z * particle.position.z;
                particle.scale = 1.0 - particle.position.z * 0.8;
                particle.color.a = 1.0 - particle.position.z as f64;
                particle.velocity = Vector3::new(
                    (random::<f32>() * 0.1 - 0.4) * particle.scale,
                    (random::<f32>() * 0.1 + 0.5) * particle.scale,
                    0.0,
                )

            }

            let snow = Model {
                mesh: Box::new(snow_particle_system),
                material: snow_material,
            };

            self.models.push(snow);
        }

        /*
        let moon = include_bytes!("resources/textures/moon.png");
        let diffuse_texture =
            texture::Texture::from_bytes(&device, &queue, moon, "moon.png").unwrap();
        let billboard = resource::create_billboard(
            0.32,
            0.32,
            Vector3::new(-1.0, -1.0, 1.0),
            diffuse_texture,
            &device,
            &layout,
        )
            .unwrap();
        self.models.push(billboard);*/
    }

    fn get_models(&self) -> &Vec<model::Model> {
        &self.models
    }
    fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, dt: std::time::Duration) {
        let mut i = 0;
        for model in &mut self.models {
            model.update(dt, queue);
            /*
            for instance in model.mesh.instances_mut().iter_mut() {
                let v_multiplier = dt.as_secs_f32() * instance.scale;
                let mut new_pos = instance.position.add(instance.velocity.mul(v_multiplier));

                //why exactly this + and - configuration? I dunno *-*
                new_pos.x = (new_pos.x - BOUNDING_BOX_X) % (2.0 * BOUNDING_BOX_X) + BOUNDING_BOX_X;
                new_pos.y = (new_pos.y + BOUNDING_BOX_Y) % (2.0 * BOUNDING_BOX_Y) - BOUNDING_BOX_Y;

                instance.position = new_pos;
                i += 1;
            }
            //model.mesh.rebuild_instance_buffer(device);
            model.mesh.update_instance_buffer(queue);
             */
        }
    }

    fn get_background_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: 0.0,
            g: 0.0,
            b: 0.01,
            a: 1.0,
        }
    }
}
