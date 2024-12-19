use std::borrow::Cow;
use wgpu::ShaderSource;

pub enum ShaderType {
    TutorialShader,
}

pub fn get(t: ShaderType) -> ShaderSource<'static> {
    match t {
        ShaderType::TutorialShader => ShaderSource::Wgsl(Cow::Borrowed(include_str!(
            "resources/shaders/tutorial_shader.wgsl"
        ))),
    }
}
