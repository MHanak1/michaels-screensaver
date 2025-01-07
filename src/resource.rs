use crate::instance::Instance;
use crate::model::{Material, Mesh, Model, ModelMesh, ModelVertex};
use crate::particle::{ParticleInstance, ParticleSystem, ParticleSystemData};
use crate::{model, texture};
use cgmath::Vector3;
use wgpu::util::DeviceExt;
use wgpu::{BindGroupLayout, Color, Device};
use crate::texture::Texture;

pub(crate) fn create_billboard(width: f32, height: f32, position: Vector3<f32>, diffuse_texture: Texture, device: &&Device, layout: &&BindGroupLayout) -> anyhow::Result<Model> {
    Ok(Model {
        mesh: Box::new(ModelMesh::create_billboard(width, height, position, device)?),
        material: Material::new(diffuse_texture, device, layout),
    })
}

pub(crate) fn create_particle_billboard(width: f32, height: f32, position: Vector3<f32>, particle_system_data: ParticleSystemData, diffuse_texture: Texture, device: &&Device, layout: &&BindGroupLayout) -> anyhow::Result<Model> {
    Ok(Model {
        mesh: Box::new(ParticleSystem::create_billboard(width, height, position, particle_system_data, device)?),
        material: Material::new(diffuse_texture, device, layout),
    })
}
