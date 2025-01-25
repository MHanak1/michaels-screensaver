use cgmath::num_traits::float::FloatCore;
use cgmath::num_traits::{clamp, Num};
use cgmath::{Vector3, Zero};
use rand::Rng;
use std::ops::{AddAssign, Div, Index, IndexMut, SubAssign};
use std::slice::SliceIndex;

pub struct InstanceContainer<T: Position2> {
    pub instances: Vec<T>,
    pub bounding_box: BoundingBox<f32>,
    pub regions_x: usize,
    pub regions_y: usize,
    pub regions: Vec<Vec<usize>>,
    //pub regions_new: Vec<usize>,
    //pub instances_sorted: Vec<usize>,
}

impl<T: Position2> InstanceContainer<T> {
    pub fn new(instances: Vec<T>, regions_x: usize, regions_y: usize) -> Self {
        InstanceContainer {
            instances,
            bounding_box: BoundingBox::new(
                Vector3::zero(),
                Vector3::zero(),
                BoundingBoxType::Ignore,
            ),
            regions_x,
            regions_y,
            regions: vec![vec![]; regions_x * regions_y],
            //regions_new: vec![0; regions_x * regions_y + 1],
            //instances_sorted: vec![0; instances.len()],
        }
    }

    pub fn instances(&self) -> &Vec<T> {
        &self.instances
    }

    pub fn instances_mut(&mut self) -> &mut Vec<T> {
        &mut self.instances
    }

    pub fn regions(&self) -> &Vec<Vec<usize>> {
        &self.regions
    }

    pub fn regions_mut(&mut self) -> &mut Vec<Vec<usize>> {
        &mut self.regions
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.instances.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.instances.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.instances.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    pub fn clear(&mut self) {
        self.instances.clear();
    }

    pub fn push(&mut self, instance: T) {
        self.instances.push(instance);
    }

    pub fn remove(&mut self, index: usize) {
        self.instances.remove(index);
    }

    pub fn get_region(&self, x: usize, y: usize) -> &Vec<usize> {
        debug_assert!(x < self.regions_x);
        debug_assert!(y < self.regions_y);
        &self.regions[y * self.regions_x + x]
    }

    pub fn get_region_mut(&mut self, x: usize, y: usize) -> &mut Vec<usize> {
        debug_assert!(x < self.regions_x);
        debug_assert!(y < self.regions_y);
        &mut self.regions[y * self.regions_x + x]
    }

    pub fn get_regions_in_range(&self, x: usize, y: usize, range: u32) -> Vec<usize> {
        debug_assert!(x < self.regions_x);
        debug_assert!(y < self.regions_y);
        let range = range as i32;
        let mut instances = vec![];
        for x1 in -range..1 + range {
            for y1 in -range..1 + range {
                let x2 = x as i32 + x1;
                let y2 = y as i32 + y1;
                if (x2 >= 0 && x2 < self.regions_x as i32)
                    && (y2 >= 0 && y2 < self.regions_y as i32)
                {
                    instances.extend(self.get_region(x2 as usize, y2 as usize));
                }
            }
        }
        instances
    }

    pub fn rebuild_regions(&mut self) {
        //self.regions = vec![; self.regions_x * self.regions_y];
        let len = self.regions.len();

        self.regions.iter_mut().for_each(|region| {
            region.clear();
        });
        //self.regions.fill(Vec::with_capacity((self.instances.len() / len) * 2));
        self.regions.resize(
            self.regions_x * self.regions_y,
            Vec::with_capacity((self.instances.len() / len) * 2),
        );
        for i in 0..self.instances.len() {
            let instance = &mut self.instances[i];

            let x: f32 = instance.x().into()
                / (2.0 * self.bounding_box.width() / self.bounding_box.height())
                + 0.5;
            let y: f32 = instance.y().into() / 2.0 + 0.5;
            self.get_region_mut(
                usize::clamp((x * self.regions_x as f32) as usize, 0, self.regions_x - 1),
                usize::clamp((y * self.regions_y as f32) as usize, 0, self.regions_y - 1),
            )
            .push(i);
        }
    }
}

impl<T: Position2, I: SliceIndex<[T]>> Index<I> for InstanceContainer<T> {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.instances.index(index)
    }
}

impl<T: Position2, I: SliceIndex<[T]>> IndexMut<I> for InstanceContainer<T> {
    fn index_mut(&mut self, index: I) -> &mut <I as SliceIndex<[T]>>::Output {
        self.instances.index_mut(index)
    }
}

pub trait Position3: Position2 {
    fn z(&self) -> impl FloatCore + Into<f32>;
}

pub trait Position2 {
    fn x(&self) -> impl FloatCore + Into<f32>;
    fn y(&self) -> impl FloatCore + Into<f32>;
}

#[derive(Debug, Clone, Copy)]
pub enum BoundingBoxType {
    Clamp,
    Modulo,
    Ignore,
    Bounce,
}

#[derive(Debug, Clone, Copy)]
pub struct BoundingBox<T: Num> {
    pub min_pos: Vector3<T>,
    pub max_pos: Vector3<T>,
    pub bound_type: BoundingBoxType,
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
    pub fn bound_type(&self) -> BoundingBoxType {
        self.bound_type
    }
}

impl<T: Num + FloatCore> BoundingBox<T> {
    pub fn new(
        position_1: Vector3<T>,
        position_2: Vector3<T>,
        bound_type: BoundingBoxType,
    ) -> BoundingBox<T> {
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
            ),
            bound_type,
        }
    }
}

impl<T: Num + From<f32> + Copy> BoundingBox<T> {
    pub fn new_with_size(
        pos: Vector3<T>,
        size_x: T,
        size_y: T,
        size_z: T,
        bound_type: BoundingBoxType,
    ) -> BoundingBox<T> {
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
            ),
            bound_type,
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

impl<T: Num + From<f32> + AddAssign + SubAssign + FloatCore> BoundingBox<T> {
    pub fn modulo_pos(&self, pos: Vector3<T>) -> Vector3<T> {
        /*
        Vector3::new(
            //((pos.x - self.min_pos.x) % (self.max_pos.x - self.min_pos.x).abs()) + self.min_pos.x,
            (pos.x - self.min_pos.x) % (self.max_pos.x - self.min_pos.x) + self.min_pos.x,
            (pos.y - self.min_pos.y) % (self.max_pos.y - self.min_pos.y) + self.min_pos.y,
            (pos.z - self.min_pos.z) % (self.max_pos.z - self.min_pos.z) + self.min_pos.z,
        )
         */
        let mut new_pos = pos;

        if pos.x < self.min_pos.x {
            new_pos.x += self.width().into()
        } else if pos.x > self.max_pos.x {
            new_pos.x -= self.width().into()
        }
        if pos.y < self.min_pos.y {
            new_pos.y += self.height().into()
        } else if pos.y > self.max_pos.y {
            new_pos.y -= self.height().into()
        }
        if pos.z < self.min_pos.z {
            new_pos.z += self.depth().into()
        } else if pos.z > self.max_pos.z {
            new_pos.z -= self.depth().into()
        }

        new_pos
    }
}

impl<
        T: Num + From<f32> + rand::distributions::uniform::SampleUniform + std::cmp::PartialOrd + Copy,
    > BoundingBox<T>
{
    pub fn random_pos(&self) -> Vector3<T> {
        Vector3::new(
            rand::thread_rng().gen_range(self.min_pos.x..=self.max_pos.x),
            rand::thread_rng().gen_range(self.min_pos.y..=self.max_pos.y),
            rand::thread_rng().gen_range(self.min_pos.z..=self.max_pos.z),
        )
    }
}
