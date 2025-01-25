use std::borrow::Cow;
use wgpu::ShaderSource;

pub enum ShaderType {
    ParticleShader,
    MeshShader,
}

impl ShaderType {
    pub fn get_source(&self) -> ShaderSource<'static> {
        match self {
            ShaderType::ParticleShader => ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "resources/shaders/particle_shader.wgsl"
            ))),
            ShaderType::MeshShader => ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "resources/shaders/model_shader.wgsl"
            ))),
        }
    }
}

#[deprecated]
pub fn get(t: ShaderType) -> ShaderSource<'static> {
    match t {
        ShaderType::ParticleShader => ShaderSource::Wgsl(Cow::Borrowed(include_str!(
            "resources/shaders/particle_shader.wgsl"
        ))),
        ShaderType::MeshShader => ShaderSource::Wgsl(Cow::Borrowed(include_str!(
            "resources/shaders/model_shader.wgsl"
        ))),
    }
}
