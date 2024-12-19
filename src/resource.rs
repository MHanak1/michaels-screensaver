use crate::model::ModelVertex;
use crate::{model, texture};
use cgmath::{Quaternion, Rotation3, Vector3};
use wgpu::Color;
use wgpu::util::DeviceExt;
use winit::dpi::Position;

pub fn create_billboard(
    width: f32,
    height: f32,
    position: Vector3<f32>,
    diffuse_texture: texture::Texture,
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
) -> anyhow::Result<model::Model> {
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

    let material = model::Material {
        diffuse_texture,
        bind_group,
    };

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

    let instances = vec![crate::Instance {
        position,
        rotation: Quaternion::from_axis_angle(Vector3::unit_z(), cgmath::Deg(0.0)),
        color: Color::WHITE,
        velocity: Vector3::new(0.0, 0.0, 0.0),
        scale: 1.0,
    }];

    let instance_data = instances
        .iter()
        .map(crate::Instance::to_raw)
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
        usage: wgpu::BufferUsages::VERTEX,
    });

    let mesh = model::Mesh {
        vertex_buffer,
        index_buffer,
        instances,
        instance_buffer,
        num_elements: indices.len() as u32,
    };

    Ok(model::Model { mesh, material })
}
