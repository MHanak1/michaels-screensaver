use crate::{model, resource, texture};
use cgmath::{Quaternion, Rad, Rotation3, Vector3};
use rand::random;
use std::ops::{Add, Mul};

const BOUNDING_BOX_X: f32 = 5.0;
const BOUNDING_BOX_Y: f32 = 2.5;

pub(crate) enum ScreenSaverType {
    Snow,
}

pub trait ScreenSaver {
    fn setup(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout);
    fn get_models(&self) -> &Vec<model::Model>;
    fn update(&mut self, device: &wgpu::Device, dt: std::time::Duration);
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
        let snow1 = include_bytes!("resources/textures/snow1.png");
        let snow2 = include_bytes!("resources/textures/snow2.png");
        let diffuse_textures = [
            texture::Texture::from_bytes(&device, &queue, snow1, "snow1.png").unwrap(),
            texture::Texture::from_bytes(&device, &queue, snow2, "snow2.png").unwrap(),
        ];
        for diffuse_texture in diffuse_textures {
            let mut snow = resource::create_billboard(
                0.05,
                0.05,
                Vector3::new(0.0, 0.0, 0.0),
                diffuse_texture,
                &device,
                &layout,
            )
            .unwrap();
            let mut new_instances = Vec::new();

            for _ in 0..5000 {
                let x = (random::<f32>() - 0.5) * BOUNDING_BOX_X * 2.0;
                let y = (random::<f32>() - 0.5) * BOUNDING_BOX_Y * 2.0;
                let depth= random::<f64>();
                let z = depth as f32* 1.5;

                let vel_x = random::<f32>() * 0.1 - 0.4;
                let vel_y = random::<f32>() * 0.1 + 0.5;

                let fg_color = wgpu::Color::WHITE;
                let bg_color = wgpu::Color {
                    r: 0.5,
                    g: 0.5,
                    b: 1.0,
                    a: 0.0,
                };
                let new_color = wgpu::Color {
                    r: fg_color.r * (1.0 - depth) + bg_color.r * depth,
                    g: fg_color.g * (1.0 - depth) + bg_color.g * depth,
                    b: fg_color.b * (1.0 - depth) + bg_color.b * depth,
                    a: fg_color.a * (1.0 - depth) + bg_color.a * depth,
                };

                new_instances.push(crate::Instance {
                    position: Vector3::new(x, y, z),
                    rotation: Quaternion::from_angle_z(Rad(0.0)),
                    color: new_color,
                    velocity: Vector3::new(vel_x, vel_y, 0.0),
                    //velocity: Vector3::new(0.0, 0.0, 0.0),
                    scale: (1.0 - z * 0.5),
                });
            }
            snow.mesh.instances = new_instances;
            snow.mesh.rebuild_instance_buffer(device);
            self.models.push(snow);
        }

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
    fn update(&mut self, device: &wgpu::Device, dt: std::time::Duration) {
        let mut i = 0;
        for mut model in &mut self.models {
            for mut instance in &mut model.mesh.instances {
                let v_multiplier = dt.as_secs_f32() * instance.scale;
                let mut new_pos = instance
                    .position
                    .add(instance.velocity.mul(v_multiplier));
                if new_pos.x < -BOUNDING_BOX_X {
                    new_pos.x += 2.0 * BOUNDING_BOX_X;
                } else if new_pos.x > BOUNDING_BOX_X {
                    new_pos.x -= 2.0 * BOUNDING_BOX_X;
                }
                if new_pos.y < -BOUNDING_BOX_Y {
                    new_pos.y += 2.0 * BOUNDING_BOX_Y;
                } else if new_pos.y > BOUNDING_BOX_Y {
                    new_pos.y -= 2.0 * BOUNDING_BOX_Y;
                }

                instance.position = new_pos;
                i += 1;
            }
            model.mesh.rebuild_instance_buffer(device);
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
