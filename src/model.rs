use crate::particle::{ParticleInstance, ParticleSystem, ParticleSystemData};
use crate::texture;
use std::ops::Range;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
use cgmath::Vector3;
#[cfg(target_arch = "wasm32")]
use web_time::Duration;
use wgpu::{Color, Queue};
use wgpu::util::DeviceExt;
use crate::instance::Instance;

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl Vertex for ModelVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub trait DrawModel<'a> {
    fn draw_mesh(&mut self, mesh: &'a Box<dyn Mesh>);
    fn draw_mesh_instanced(&mut self, mesh: &'a Box<dyn Mesh>, instances: Range<u32>);
}
impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(&mut self, mesh: &'b Box<dyn Mesh>) {
        self.draw_mesh_instanced(mesh, 0..1);
    }

    fn draw_mesh_instanced(&mut self, mesh: &'b Box<dyn Mesh>, instances: Range<u32>) {
        mesh.draw_self_instanced(self, instances);
    }
}

pub struct Model {
    pub mesh: Box<dyn Mesh>,
    pub material: Material,
}

impl Model {
    pub(crate) fn update(&mut self, delta_t: Duration, queue: &Queue) {
        self.mesh.update(delta_t, queue);
    }
}

pub struct Material {
    pub diffuse_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

impl Material {
    pub fn new(
        diffuse_texture: texture::Texture,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> Material {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: None,
        });

        Material {
            diffuse_texture,
            bind_group,
        }
    }
}

pub trait Mesh: DrawMesh {
    fn rebuild_instance_buffer(&mut self, device: &wgpu::Device);
    fn update_instance_buffer(&mut self, queue: &Queue);
    fn instance_count(&self) -> usize;
    fn instances_mut(&mut self) -> &mut Vec<Box<ParticleInstance>>;
    fn set_instances(&mut self, instances: Vec<Box<ParticleInstance>>);
    fn update(&mut self, delta_t: Duration, queue: &Queue);
}

pub struct ModelMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub instances: Vec<Box<ParticleInstance>>,
    pub num_elements: u32,
}

impl ModelMesh {
    pub fn create_billboard(
        width: f32,
        height: f32,
        position: Vector3<f32>,
        device: &wgpu::Device,
    ) -> anyhow::Result<ModelMesh> {
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

        Ok(ModelMesh {
            vertex_buffer,
            index_buffer,
            instances,
            instance_buffer,
            num_elements: indices.len() as u32,
        })
    }
}

impl Mesh for ModelMesh {
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
        //do nothing
    }
}

impl DrawMesh for ModelMesh {
    fn draw_self_instanced(&self, pass: &mut wgpu::RenderPass, instances: Range<u32>) {
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.draw_indexed(0..self.num_elements, 0, instances);
    }
}

pub trait DrawMesh {
    fn draw_self_instanced(&self, pass: &mut wgpu::RenderPass, instances: Range<u32>);
}
