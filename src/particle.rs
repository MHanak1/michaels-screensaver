use crate::util::BoundingBox;
use crate::instance::{Instance, InstanceRaw};
use crate::model::{DrawMesh, Mesh, ModelVertex};
use std::ops::{Add, Mul, Range};
use std::time::Duration;
use cgmath::Vector3;
use wgpu::util::DeviceExt;
use wgpu::{Color, Queue};
use rand::random;

pub struct ParticleSystemData {
    domain: BoundingBox<f32>,
}
impl ParticleSystemData {
    pub fn new(domain: BoundingBox<f32>) -> Self {
        ParticleSystemData {
            domain
        }
    }
}

pub struct ParticleSystem {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub instances: Vec<Box<ParticleInstance>>,
    pub particle_system_data: ParticleSystemData,
    pub num_elements: u32,
}

impl ParticleSystem {
    pub fn create_billboard(
        width: f32,
        height: f32,
        position: Vector3<f32>,
        particle_system_data: ParticleSystemData,
        device: &wgpu::Device,
    ) -> anyhow::Result<ParticleSystem> {
        let vertices = &[
            ModelVertex {
                position: [-width / 2.0, -height / 2.0, 0.0],
                tex_coords: [1.0, 0.0],
            },
            ModelVertex {
                position: [width / 2.0, -height / 2.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            ModelVertex {
                position: [-width / 2.0, height / 2.0, 0.0],
                tex_coords: [1.0, 1.0],
            },
            ModelVertex {
                position: [width / 2.0, height / 2.0, 0.0],
                tex_coords: [0.0, 1.0],
            },
        ];

        let indices: &[u32] = &[0, 1, 2, 1, 3, 2];

        let instances = vec![Box::new(ParticleInstance {
            position,
            color: Color::WHITE,
            velocity: Vector3::new(0.0, 0.0, 0.0),
            scale: 1.0,
        })];

        let instance_data = instances
            .iter()
            .map(|particle_instance: &Box<ParticleInstance>| {
                ParticleInstance::to_raw(particle_instance)
            })
            .collect::<Vec<_>>();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&vertices.as_slice()),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Ok(ParticleSystem {
            vertex_buffer,
            index_buffer,
            instances,
            instance_buffer,
            num_elements: indices.len() as u32,
            particle_system_data,
        })
    }
    pub fn populate_random(&mut self, instance_count: usize, device: &wgpu::Device) {
        self.instances = Vec::with_capacity(instance_count);


        for i in 0..7500 {
            let mut position = self.particle_system_data.domain.random_pos();

            let new_color = wgpu::Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            };

            self.instances.push(Box::new(ParticleInstance {
                position,
                color: new_color,
                velocity: Vector3::new(0.0, 0.0, 0.0),
                //velocity: Vector3::new(0.0, 0.0, 0.0),
                scale: 1.0,
            }));
        }
        self.rebuild_instance_buffer(device);
    }
}

impl Mesh for ParticleSystem {
    fn rebuild_instance_buffer(&mut self, device: &wgpu::Device) {
        let instance_data = self
            .instances
            .iter()
            .map(|particle_instance: &Box<ParticleInstance>| {
                ParticleInstance::to_raw(particle_instance)
            })
            .collect::<Vec<_>>();

        self.instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: wgpu::Label::from("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
    }
    fn update_instance_buffer(&mut self, queue: &Queue) {
        let instance_data = self
            .instances
            .iter()
            .map(|particle_instance: &Box<ParticleInstance>| {
                ParticleInstance::to_raw(particle_instance)
            })
            .collect::<Vec<_>>();

        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&instance_data),
        );
    }

    fn instance_count(&self) -> usize {
        self.instances.len()
    }

    fn instances_mut(&mut self) -> &mut Vec<Box<ParticleInstance>> {
        &mut self.instances
    }

    fn set_instances(&mut self, instances: Vec<Box<ParticleInstance>>) {
        self.instances = instances;
    }

    fn update(&mut self, delta_t: Duration, queue: &Queue) {
        for instance in self.instances.iter_mut() {
            let v_multiplier = delta_t.as_secs_f32() * instance.scale;
            let mut new_pos = instance.position.add(instance.velocity.mul(v_multiplier));

            instance.position = self.particle_system_data.domain.modulo_pos(new_pos);
        }
        //model.mesh.rebuild_instance_buffer(device);
        self.update_instance_buffer(queue);
    }
}

impl DrawMesh for ParticleSystem {
    fn draw_self_instanced(&self, pass: &mut wgpu::RenderPass, instances: Range<u32>) {
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.draw_indexed(0..self.num_elements, 0, instances);
    }
}

pub struct ParticleInstance {
    pub(crate) position: cgmath::Vector3<f32>,
    //rotation: cgmath::Quaternion<f32>,
    pub(crate) color: wgpu::Color,
    pub(crate) velocity: cgmath::Vector3<f32>, //in case i wanted to update the position through a compute shader (don't know how yet)
    pub(crate) scale: f32,
}

impl Instance for ParticleInstance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            position: self.position.into(),
            //model: Matrix4::from_translation(Vector3::zero()).into(),
            color: [
                self.color.r as f32,
                self.color.g as f32,
                self.color.b as f32,
                self.color.a as f32,
            ],
            //velocity: self.velocity.into(),
            scale: self.scale,
        }
    }
}
