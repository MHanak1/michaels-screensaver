// Vertex shader
struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct InstanceInput {
    @location(3) color: vec4<f32>,
    //@location(10) velocity: vec3<f32>,
    @location(4) scale: f32,
    @location(5) position: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    //out.clip_position[3] *= 0.01;
    out.clip_position = camera.view_proj * vec4<f32>(instance.position + model.position * instance.scale, 1.0); // 2.
    out.color = instance.color;
    return out;
}

// Fragment Shader

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var out = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    out *= in.color;
    if out[3] == 0 {
        discard;
    }
    return out;
}



