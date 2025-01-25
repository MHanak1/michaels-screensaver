use crate::model::{Material, Model, ModelMesh};
use crate::particle::{ParticleSystem, ParticleSystemData};
use crate::texture::Texture;
use cgmath::Vector3;
use wgpu::{BindGroupLayout, Device};

pub(crate) fn create_billboard(
    width: f32,
    height: f32,
    position: Vector3<f32>,
    diffuse_texture: Texture,
    device: &&Device,
    layout: &&BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
) -> anyhow::Result<Model> {
    Ok(Model {
        mesh: Box::new(ModelMesh::create_billboard(width, height, position, device)),
        material: Material::new(diffuse_texture, device, layout, pipeline),
    })
}

pub(crate) fn create_particle_billboard(
    width: f32,
    height: f32,
    position: Vector3<f32>,
    particle_system_data: ParticleSystemData,
    diffuse_texture: Texture,
    device: &&Device,
    layout: &&BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
) -> anyhow::Result<Model> {
    Ok(Model {
        mesh: Box::new(ParticleSystem::create_billboard(
            width,
            height,
            position,
            particle_system_data,
            device,
        )),
        material: Material::new(diffuse_texture, device, layout, pipeline),
    })
}

pub(crate) fn load_mesh(
    width: f32,
    height: f32,
    position: Vector3<f32>,
    diffuse_texture: Texture,
    device: &&Device,
    layout: &&BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
) -> anyhow::Result<Model> {
    Ok(Model {
        mesh: Box::new(ModelMesh::create_billboard(width, height, position, device)),
        material: Material::new(diffuse_texture, device, layout, pipeline),
    })
}
