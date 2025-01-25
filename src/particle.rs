#![allow(dead_code)]

use crate::instance::{LayoutDescriptor, ToRaw};
use crate::model::{DrawMesh, Mesh, ModelVertex};
use crate::util::pos::{BoundingBox, BoundingBoxType, InstanceContainer, Position2, Position3};
use cgmath::{Vector2, Vector3, Zero};
use std::ops::{Add, Mul, Range};
use std::time::Duration;
use wgpu::util::DeviceExt;
use wgpu::Queue;

#[derive(Debug, Clone, Copy)]
pub struct ParticleData {
    pub velocity: Vector3<f32>,
    pub collider: Option<Vector2<f32>>,
}

pub struct ParticleSystemData {
    pub domain: BoundingBox<f32>,
}
impl ParticleSystemData {
    pub fn new(domain: BoundingBox<f32>) -> Self {
        ParticleSystemData { domain }
    }
}

pub struct ParticleSystem {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub instances: InstanceContainer<ParticleInstance>,
    pub particle_data: Vec<ParticleData>,
    pub particle_system_data: ParticleSystemData,
    pub num_elements: u32,
}

impl ParticleSystem {
    pub fn create_billboard(
        width: f32,
        height: f32,
        _position: Vector3<f32>,
        particle_system_data: ParticleSystemData,
        device: &wgpu::Device,
    ) -> ParticleSystem {
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

        let instances = vec![];
        let particle_data = vec![ParticleData {
            velocity: Vector3::zero(),
            collider: Option::from(Vector2::new(width, height)), //cheeky hack to transfer the width and height to the population routine
        }];

        let instance_data = instances
            .iter()
            .map(|particle_instance: &ParticleInstance| ParticleInstance::to_raw(particle_instance))
            .collect::<Vec<_>>();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(vertices.as_slice()),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        ParticleSystem {
            vertex_buffer,
            index_buffer,
            instances: InstanceContainer::new(instances, 1, 1),
            particle_data,
            instance_buffer,
            num_elements: indices.len() as u32,
            particle_system_data,
        }
    }
    pub fn populate_random(&mut self, instance_count: usize, device: &wgpu::Device) {
        for _ in 0..instance_count {
            let position = self.particle_system_data.domain.random_pos();

            let new_color = wgpu::Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            };

            self.instances.push(ParticleInstance {
                position,
                color: new_color,
                //velocity: Vector3::new(0.0, 0.0, 0.0),
                scale: 1.0,
                age: Duration::new(0, 0),
            });
            self.particle_data.push(ParticleData {
                velocity: Vector3::zero(),
                collider: self.particle_data[0].collider,
            });
        }
        self.rebuild_instance_buffer(device);
    }
}

impl Mesh for ParticleSystem {
    fn rebuild_instance_buffer(&mut self, device: &wgpu::Device) {
        let instance_data = self
            .instances
            .iter()
            .map(|particle_instance: &ParticleInstance| ParticleInstance::to_raw(particle_instance))
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
            .map(|particle_instance: &ParticleInstance| ParticleInstance::to_raw(particle_instance))
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

    fn update(&mut self, delta_t: Duration, queue: &Queue) {
        for i in 0..self.instances.len() {
            let instance = &mut self.instances[i];
            let data = &mut self.particle_data[i];
            instance.update(delta_t);

            if !instance.position[0].is_normal() {
                instance.position = self.particle_system_data.domain.random_pos();
            }

            match self.particle_system_data.domain.bound_type() {
                BoundingBoxType::Clamp => {
                    instance.position = self.particle_system_data.domain.clamp_pos(
                        instance
                            .position
                            .add(data.velocity.mul(delta_t.as_secs_f32())),
                    );
                }
                BoundingBoxType::Modulo => {
                    instance.position = self.particle_system_data.domain.modulo_pos(
                        instance
                            .position
                            .add(data.velocity.mul(delta_t.as_secs_f32())),
                    );
                }
                BoundingBoxType::Bounce => {
                    let collider = match data.collider {
                        None => Vector2::zero(),
                        Some(collider) => collider,
                    };
                    if self.particle_system_data.domain.min_pos.x - instance.position.x
                        > -instance.scale * collider.x / 2.0
                    {
                        data.velocity.x = data.velocity.x.abs();
                    } else if self.particle_system_data.domain.max_pos.x - instance.position.x
                        < instance.scale * collider.x / 2.0
                    {
                        data.velocity.x = -data.velocity.x.abs();
                    }
                    if self.particle_system_data.domain.min_pos.y - instance.position.y
                        > -instance.scale * collider.y / 2.0
                    {
                        data.velocity.y = data.velocity.y.abs();
                    } else if self.particle_system_data.domain.max_pos.y - instance.position.y
                        < instance.scale * collider.y / 2.0
                    {
                        data.velocity.y = -data.velocity.y.abs();
                    }
                    if self.particle_system_data.domain.min_pos.z - instance.position.z > 0.0 {
                        data.velocity.z = data.velocity.z.abs();
                    } else if self.particle_system_data.domain.max_pos.z - instance.position.z < 0.0
                    {
                        data.velocity.z = -data.velocity.z.abs();
                    }
                    instance.position = self.particle_system_data.domain.clamp_pos(
                        instance
                            .position
                            .add(data.velocity.mul(delta_t.as_secs_f32())),
                    );
                }
                BoundingBoxType::Ignore => {
                    instance.position = instance
                        .position
                        .add(data.velocity.mul(delta_t.as_secs_f32()));
                }
            }
            instance.age += delta_t;
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

#[derive(Debug, Clone, Copy)]
pub struct ParticleInstance {
    pub(crate) position: cgmath::Vector3<f32>,
    //rotation: cgmath::Quaternion<f32>,
    pub(crate) color: wgpu::Color,
    pub(crate) scale: f32,
    pub(crate) age: Duration,
}

impl ToRaw for ParticleInstance {
    fn to_raw(&self) -> ParticleInstanceRaw {
        ParticleInstanceRaw {
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

impl ParticleInstance {
    pub fn update(&mut self, delta_time: Duration) {
        self.age = self.age.add(delta_time);
    }
}

impl Position2 for ParticleInstance {
    fn x(&self) -> f32 {
        self.position.x
    }

    fn y(&self) -> f32 {
        self.position.y
    }
}

impl Position3 for ParticleInstance {
    fn z(&self) -> f32 {
        self.position.z
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleInstanceRaw {
    pub(crate) color: [f32; 4],
    //velocity: [f32; 3],
    pub(crate) scale: f32,
    pub(crate) position: [f32; 3],
}

impl LayoutDescriptor for ParticleInstanceRaw {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ParticleInstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0 as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
