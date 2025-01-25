#![allow(dead_code)]

use crate::instance::{Instance, LayoutDescriptor, ToRaw};
use crate::util::pos::{Position2, Position3};
use crate::{model, texture};
use cgmath::{Point2, Point3, Quaternion, Rotation3, Vector3};
use downcast_rs::Downcast;
use std::io::{BufReader, Cursor, Read};
use std::ops::{Add, Range};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(target_arch = "wasm32")]
use web_time::Duration;
use wgpu::util::DeviceExt;
use wgpu::{Color, Queue, RenderPipeline};
use winit::dpi::Position;
use crate::util::model::DDDModel;

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
    fn draw_mesh(&mut self, mesh: &'a dyn Mesh);
    fn draw_mesh_instanced(&mut self, mesh: &'a dyn Mesh, instances: Range<u32>);
}
impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(&mut self, mesh: &'b dyn Mesh) {
        self.draw_mesh_instanced(mesh, 0..1);
    }

    fn draw_mesh_instanced(&mut self, mesh: &'b dyn Mesh, instances: Range<u32>) {
        mesh.draw_self_instanced(self, instances);
    }
}

pub struct Model {
    pub mesh: Box<dyn Mesh>,
    pub material: Material,
}

impl Model {
    pub fn load(
        model: DDDModel,
        position: Vector3<f32>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        pipeline: RenderPipeline,
    ) -> anyhow::Result<Model> {
        //let obj_text  =model.get().0;
        //let obj_text = include_str!("resources/models/apple.obj");
        let obj_text = model.get().0;
        let obj_cursor = Cursor::new(obj_text);
        let mut obj_reader = BufReader::new(obj_cursor);

        let (models, _)= tobj::load_obj_buf(&mut obj_reader, &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        }, |_| {
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new("")))
        })?;

        let diffuse_texture = texture::Texture::from_bytes(
            device,
            queue,
            &*model.get().1,
            "",
        )?;

        let material = Material::new(diffuse_texture, device, layout, pipeline);

        let mesh = {
            let vertices = (0..models[0].mesh.positions.len() / 3)
                .map(|i| model::ModelVertex {
                    position: [
                        models[0].mesh.positions[i * 3],
                        models[0].mesh.positions[i * 3 + 1],
                        models[0].mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [
                        models[0].mesh.texcoords[i * 2],
                        1.0 - models[0].mesh.texcoords[i * 2 + 1],
                    ],
                })
                .collect::<Vec<_>>();

            let instances: Vec<ModelInstance> = vec![
                ModelInstance {
                    position,
                    ..Default::default()
                }
            ];

            let instance_data = instances
                .iter()
                .map(|model_instance: &ModelInstance| model_instance.to_raw())
                .collect::<Vec<_>>();

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&"Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&"Index Buffer"),
                contents: bytemuck::cast_slice(&models[0].mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

            ModelMesh {
                vertex_buffer,
                index_buffer,
                instance_buffer,
                instances,
                num_elements: models[0].mesh.indices.len() as u32,
            }
        };

        Ok(model::Model {
            mesh: Box::new(mesh),
            material,
        })
    }

    pub(crate) fn update(&mut self, delta_t: Duration, queue: &Queue) {
        self.mesh.update(delta_t, queue);
    }
}

pub struct Material {
    pub pipeline: wgpu::RenderPipeline,
    pub diffuse_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

impl Material {
    pub fn new(
        diffuse_texture: texture::Texture,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        pipeline: wgpu::RenderPipeline,
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
            pipeline,
        }
    }
}

pub trait Mesh: DrawMesh + Downcast {
    fn rebuild_instance_buffer(&mut self, device: &wgpu::Device);
    fn update_instance_buffer(&mut self, queue: &Queue);
    fn instance_count(&self) -> usize;
    //fn set_instances(&mut self, instances: Vec<Box<dyn Instance>>);
    fn update(&mut self, _delta_t: Duration, _queue: &Queue);
}

pub trait Instanced {
    fn instances(&mut self) -> &mut Vec<impl Instance>;
}

pub struct ModelMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub instances: Vec<ModelInstance>,
    pub num_elements: u32,
}

impl ModelMesh {
    pub fn create_billboard(
        width: f32,
        height: f32,
        position: Vector3<f32>,
        device: &wgpu::Device,
    ) -> impl Mesh {
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

        let instances: Vec<ModelInstance> = vec![ModelInstance {
            position,
            ..Default::default()
        }];

        let instance_data = instances
            .iter()
            .map(|model_instance: &ModelInstance| model_instance.to_raw())
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

        ModelMesh {
            vertex_buffer,
            index_buffer,
            instances,
            instance_buffer,
            num_elements: indices.len() as u32,
        }
    }
}

impl Mesh for ModelMesh {
    fn rebuild_instance_buffer(&mut self, device: &wgpu::Device) {
        let instance_data = self
            .instances
            .iter()
            .map(|instance: &ModelInstance| instance.to_raw())
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
            .map(|instance: &ModelInstance| instance.to_raw())
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
        for instance in self.instances.iter_mut() {
            instance.update(delta_t)
        }
        self.update_instance_buffer(queue);
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

#[derive(Debug, Clone, Copy)]
pub struct ModelInstance {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
    //rotation: cgmath::Quaternion<f32>,
    pub scale: f32,
    pub age: Duration,
}

impl Default for ModelInstance {
    fn default() -> Self {
        Self {
            position: Vector3::from([0.0, 0.0, 0.0]),
            rotation: Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
            scale: 1.0,
            age: Default::default(),
        }
    }
}

impl Instance for ModelInstance {
    fn update(&mut self, delta_time: Duration) {
        self.age = self.age.add(delta_time);
    }
}

impl ToRaw for ModelInstance {
    fn to_raw(&self) -> ModelInstanceRaw {
        ModelInstanceRaw {
            //velocity: self.velocity.into(),
            scale: self.scale,
            position: self.position.into(),
            model: (cgmath::Matrix4::from_translation(self.position)
                * cgmath::Matrix4::from(self.rotation))
            .into(),
        }
    }
}

impl Position2 for ModelInstance {
    fn x(&self) -> f32 {
        self.position.x
    }

    fn y(&self) -> f32 {
        self.position.y
    }
}

impl Position3 for ModelInstance {
    fn z(&self) -> f32 {
        self.position.z
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelInstanceRaw {
    //velocity: [f32; 3],
    pub scale: f32,
    pub position: [f32; 3],
    pub model: [[f32; 4]; 4],
}

impl LayoutDescriptor for ModelInstanceRaw {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelInstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                //scale
                wgpu::VertexAttribute {
                    offset: 0 as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
                //position
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 1]>() as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x3,
                },

                //transform matrix
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
