use crate::texture;
use std::ops::Range;
use wgpu::util::DeviceExt;

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
    fn draw_mesh(&mut self, mesh: &'a Mesh);
    fn draw_mesh_instanced(&mut self, mesh: &'a Mesh, instances: Range<u32>);
}
impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(&mut self, mesh: &'b Mesh) {
        self.draw_mesh_instanced(mesh, 0..1);
    }

    fn draw_mesh_instanced(&mut self, mesh: &'b Mesh, instances: Range<u32>) {
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_vertex_buffer(1, mesh.instance_buffer.slice(..));
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }
}

pub struct Model {
    pub mesh: Mesh,
    pub material: Material,
}

pub struct Material {
    pub diffuse_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instances: Vec<crate::Instance>,
    pub instance_buffer: wgpu::Buffer,
    pub num_elements: u32,
}

impl Mesh {
    pub fn rebuild_instance_buffer(&mut self, device: &wgpu::Device) {
        let instance_data = self
            .instances
            .iter()
            .map(crate::Instance::to_raw)
            .collect::<Vec<_>>();
        self.instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: wgpu::Label::from("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
    }
}
