use cgmath::num_traits::{clamp, Num};
use cgmath::num_traits::float::FloatCore;
use cgmath::num_traits::real::Real;
use cgmath::Vector3;
use rand::Rng;

pub struct BoundingBox<T: Num> {
    pub min_pos: Vector3<T>,
    pub max_pos: Vector3<T>,
}

impl<T: Num + Copy> BoundingBox<T> {
    pub fn width(&self) -> T {
        self.max_pos.x - self.min_pos.x
    }
    pub fn height(&self) -> T {
        self.max_pos.y - self.min_pos.y
    }
    pub fn depth(&self) -> T {
        self.max_pos.z - self.min_pos.z
    }
}
impl<T: Num + FloatCore> BoundingBox<T> {
    pub fn new(position_1: Vector3<T>, position_2: Vector3<T>) -> BoundingBox<T> {
        BoundingBox {
            min_pos: Vector3::new(
                T::min(position_1.x, position_2.x),
                T::min(position_1.y, position_2.y),
                T::min(position_1.z, position_2.z),
            ),
            max_pos: Vector3::new(
                T::max(position_1.x, position_2.x),
                T::max(position_1.y, position_2.y),
                T::max(position_1.z, position_2.z),
            )
        }
    }
}
impl<T: Num + From<f32> + Copy> BoundingBox<T> {
    pub fn new_with_size(pos: Vector3<T>, size_x: T, size_y: T, size_z: T) -> BoundingBox<T> {
        BoundingBox {
            min_pos: Vector3::new(
                pos.x - size_x.div(2.0.into()),
                pos.y - size_y.div(2.0.into()),
                pos.z - size_z.div(2.0.into()),
            ),
            max_pos: Vector3::new(
                pos.x + size_x.div(2.0.into()),
                pos.y + size_y.div(2.0.into()),
                pos.z + size_z.div(2.0.into()),
            )
        }
    }
}
impl<T: Num + From<f32> + std::cmp::PartialOrd + Copy> BoundingBox<T> {
    pub fn clamp_pos(&self, pos: Vector3<T>) -> Vector3<T> {
        Vector3::new(
            clamp(pos.x, self.min_pos.x, self.max_pos.x),
            clamp(pos.y, self.min_pos.y, self.max_pos.y),
            clamp(pos.z, self.min_pos.z, self.max_pos.z),
        )
    }
}
impl<T: Num + From<f32> + FloatCore> BoundingBox<T> {
    pub fn modulo_pos(&self, pos: Vector3<T>) -> Vector3<T> {
        Vector3::new(
            ((pos.x + self.min_pos.x) % (self.max_pos.x - self.min_pos.x).abs()) - self.min_pos.x,
            ((pos.y - self.min_pos.y) % (self.max_pos.y - self.min_pos.y).abs()) + self.min_pos.y,
            ((pos.z - self.min_pos.z) % (self.max_pos.z - self.min_pos.z).abs()) + self.min_pos.z,
        )
    }
}

impl<T: Num + From<f32> + rand::distributions::uniform::SampleUniform + std::cmp::PartialOrd + Copy> BoundingBox<T> {
    pub fn random_pos(&self) -> Vector3<T> {
        Vector3::new(
            rand::thread_rng().gen_range(self.min_pos.x..=self.max_pos.x),
            rand::thread_rng().gen_range(self.min_pos.y..=self.max_pos.y),
            rand::thread_rng().gen_range(self.min_pos.z..=self.max_pos.z),
        )
    }
}