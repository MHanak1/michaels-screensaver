#![allow(dead_code)]

use crate::util::{Position2, Position3};
use std::ops::Add;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct Instance {
    pub(crate) position: cgmath::Vector3<f32>,
    //rotation: cgmath::Quaternion<f32>,
    pub(crate) color: wgpu::Color,
    pub(crate) scale: f32,
    pub(crate) age: Duration,
}

impl Instance {
    pub(crate) fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
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

    pub fn update(&mut self, delta_time: Duration) {
        self.age = self.age.add(delta_time);
    }
}

impl Position2 for Instance {
    fn x(&self) -> f32 {
        self.position.x
    }

    fn y(&self) -> f32 {
        self.position.y
    }
}

impl Position3 for Instance {
    fn z(&self) -> f32 {
        self.position.z
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub(crate) color: [f32; 4],
    //velocity: [f32; 3],
    pub(crate) scale: f32,
    pub(crate) position: [f32; 3],
}

impl InstanceRaw {
    pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
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
