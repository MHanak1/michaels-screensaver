#![allow(dead_code)]

use crate::util::pos::Position3;
use std::time::Duration;

pub trait Instance: ToRaw + Position3 {
    fn update(&mut self, delta_time: Duration);
}

pub trait ToRaw {
    fn to_raw(&self) -> impl LayoutDescriptor;
}

pub trait LayoutDescriptor {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}
