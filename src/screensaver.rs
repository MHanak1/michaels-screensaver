use crate::configurator::Configurator;
use crate::model::{DrawModel, Material, Mesh, Model};
use crate::particle::{ParticleSystem, ParticleSystemData};
use crate::util::{BoundingBox, BoundingBoxType};
use crate::{texture, util, State};
use cgmath::{InnerSpace, MetricSpace, Vector3};
use prisma::{Hsv, Rgb};
use rand::prelude::SliceRandom;
use rand::random;
use std::ops::{AddAssign, MulAssign};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};
use wgpu::Color;
use winit::dpi::Size;

#[derive(Debug, Clone, PartialEq, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) enum ScreenSaverType {
    Snow,
    Balls,
}

impl ToString for ScreenSaverType {
    fn to_string(&self) -> String {
        match self {
            ScreenSaverType::Snow => "snow".to_string(),
            ScreenSaverType::Balls => "balls".to_string(),
        }
    }
}
/*
lazy_static! {
    pub static ref SCREEN_SAVER_NAMES: HashMap<String, ScreenSaverType> =
        HashMap::from([
            ("snow".to_string(), ScreenSaverType::Snow),
            ("circles".to_string(), ScreenSaverType::Balls),
        ]);
}
*/
pub trait ScreenSaver {
    fn new(config: Configurator) -> Self
    where
        Self: Sized;
    fn setup(
        &mut self,
        size: Size,
        config: &Configurator,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    );
    fn update(
        &mut self,
        size: Size,
        config: &Configurator,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        dt: Duration,
    );
    fn resize(&mut self, old_ratio: f32, new_ratio: f32);
    fn get_background_color(&self) -> wgpu::Color;
    fn handle_input(&mut self, position: [f32; 2], id: u64, active: bool) -> bool;
    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, state: &State<'_>);
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub(crate) enum BallColorMode {
    Random,
    Color,
    Infection,
    Temperature,
}

impl ToString for BallColorMode {
    fn to_string(&self) -> String {
        match self {
            BallColorMode::Random => "random".to_string(),
            BallColorMode::Color => "color".to_string(),
            BallColorMode::Infection => "infection".to_string(),
            BallColorMode::Temperature => "temperature".to_string(),
        }
    }
}

pub struct BallScreenSaver {
    balls: Vec<Model>,
    inputs: [Option<[f32; 2]>; 6],
    first_input_handled: bool,
    actual_ball_speed: f32,
    //config
    color: Color,
    old_config: Configurator,
}
impl ScreenSaver for BallScreenSaver {
    fn new(config: Configurator) -> BallScreenSaver
    where
        Self: Sized,
    {
        Self {
            balls: vec![],
            inputs: [None; 6],
            first_input_handled: false,
            color: util::color_from_hex(config.color.to_hex()).unwrap(),
            actual_ball_speed: config.ball_speed,
            old_config: config,
        }
    }
    fn setup(
        &mut self,
        size: Size,
        config: &Configurator,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) {
        let ratio = if size.to_logical::<f32>(1.0).width > 1.0 {
            size.to_logical::<f32>(1.0).width / size.to_logical::<f32>(1.0).height
        } else {
            1.0
        };

        let circle_texture = include_bytes!("resources/textures/circle16.png");
        let diffuse_texture =
            texture::Texture::from_bytes(device, queue, circle_texture, "circle16.png").unwrap();
        let mut particle_system = ParticleSystem::create_billboard(
            0.16,
            0.16,
            Vector3::new(0.0, 0.0, 0.0),
            ParticleSystemData::new(BoundingBox::new_with_size(
                //Vector3::new(0.0, 0.0, 0.55),
                Vector3::new(0.0, 0.0, 0.0),
                2.0 * ratio,
                2.0,
                //1.00,
                0.0,
                BoundingBoxType::Bounce,
            )),
            device,
        );

        let material = Material::new(diffuse_texture, device, layout);

        particle_system.populate_random(config.ball_count, device);

        let infection_starting_color = util::random_color();

        for i in 0..particle_system.instances.len() {
            let instance = &mut particle_system.instances[i];
            let data = &mut particle_system.particle_data[i];

            let mut move_vector = Vector3::new(
                random::<f32>() - 0.5,
                random::<f32>() - 0.5,
                //random::<f32>() - 0.5,
                0.0,
            );
            move_vector = move_vector.normalize() * config.ball_speed;

            data.velocity = move_vector;

            match config.color_mode {
                BallColorMode::Random => {
                    instance.color = util::random_color();
                }
                BallColorMode::Color => {
                    instance.color = self.color;
                }
                BallColorMode::Infection => {
                    if i == 0 {
                        self.color = util::random_distinct_color(infection_starting_color);
                        instance.color = self.color;
                    } else {
                        instance.color = infection_starting_color;
                    }
                }
                _ => {
                    instance.color = Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    };
                }
            }
            instance.scale = config.ball_size;
        }

        let balls = Model {
            mesh: Box::new(particle_system),
            material,
        };

        self.balls.push(balls);
    }

    fn update(
        &mut self,
        size: Size,
        config: &Configurator,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        dt: Duration,
    ) {
        let ratio = size.to_logical::<f32>(1.0).width / size.to_logical::<f32>(1.0).height;
        //Note: this only is non-zero later if self.correct_ball_velocity is true
        let mut total_velocity = 0.0;

        let mut infected_balls = 0;
        let infection_starting_color = util::random_color();

        for model in &mut self.balls {
            //get (ParticleSystem)(Object) idiot
            if let Some(particle_system) = model.mesh.as_any_mut().downcast_mut::<ParticleSystem>()
            {
                if *config != self.old_config {
                    println!("config changed");
                    let mut should_rebuild_instance_buffer = false;

                    if config.ball_speed != self.old_config.ball_speed {
                        //redo the calculation because i am not sure if actual_ball_velocity is always calculated
                        let mut total_v = 0.0;
                        for data in particle_system.particle_data.iter() {
                            total_v += data.velocity.magnitude();
                        }
                        let avg_v: f32 = total_v / particle_system.particle_data.len() as f32;
                        if avg_v.is_normal() {
                            for data in particle_system.particle_data.iter_mut() {
                                data.velocity.mul_assign(config.ball_speed / avg_v);
                            }
                        }
                    }

                    if config.ball_count != self.old_config.ball_count {
                        let delta = config.ball_count as i32 - self.old_config.ball_count as i32;
                        if delta > 0 {
                            particle_system.populate_random(delta.try_into().unwrap(), device);

                            for i in self.old_config.ball_count..particle_system.instances.len() {
                                let instance = &mut particle_system.instances[i];
                                let data = &mut particle_system.particle_data[i];

                                let mut move_vector = Vector3::new(
                                    random::<f32>() - 0.5,
                                    random::<f32>() - 0.5,
                                    //random::<f32>() - 0.5,
                                    0.0,
                                );
                                move_vector = move_vector.normalize() * config.ball_speed;

                                data.velocity = move_vector;

                                match config.color_mode {
                                    BallColorMode::Random => {
                                        instance.color = util::random_distinct_color(self.color);
                                    }
                                    BallColorMode::Color => {
                                        instance.color = self.color;
                                    }
                                    BallColorMode::Infection => {
                                        if i == 0 {
                                            instance.color = self.color;
                                        } else {
                                            instance.color = infection_starting_color;
                                        }
                                    }
                                    _ => {
                                        instance.color = Color {
                                            r: 1.0,
                                            g: 1.0,
                                            b: 1.0,
                                            a: 1.0,
                                        };
                                    }
                                }
                                instance.scale = config.ball_size;
                            }
                        } else {
                            particle_system
                                .instances
                                .instances
                                .truncate(config.ball_count);
                            particle_system.particle_data.truncate(config.ball_count);
                        }

                        should_rebuild_instance_buffer = true;
                    }

                    if config.ball_size != self.old_config.ball_size {
                        for instance in particle_system.instances.instances.iter_mut() {
                            instance.scale = config.ball_size;
                        }
                    }

                    if config.color_mode != self.old_config.color_mode
                        || config.color != self.old_config.color
                    {
                        self.color = util::color_from_hex(config.color.to_hex()).unwrap();
                        let infection_starting_color = util::random_color();
                        for i in 0..particle_system.instances.instances.len() {
                            let instance = &mut particle_system.instances[i];
                            match config.color_mode {
                                BallColorMode::Random => {
                                    instance.color = util::random_distinct_color(self.color);
                                }
                                BallColorMode::Color => {
                                    instance.color = self.color;
                                }
                                BallColorMode::Infection => {
                                    if i == 0 {
                                        self.color = util::random_color();
                                        instance.color = self.color;
                                    } else {
                                        instance.color = infection_starting_color
                                    }
                                }
                                _ => {
                                    instance.color = Color {
                                        r: 1.0,
                                        g: 1.0,
                                        b: 1.0,
                                        a: 1.0,
                                    };
                                }
                            }
                        }
                    }

                    if config.show_density != self.old_config.show_density {
                        for instance in particle_system.instances.iter_mut() {
                            if !config.show_density {
                                instance.color.a = 1.0;
                            }
                        }
                    }

                    if should_rebuild_instance_buffer {
                        particle_system.rebuild_instance_buffer(device);
                    }

                    self.old_config = *config;
                }

                particle_system.instances.regions_x =
                    (1.0 * ratio / (0.16 * config.ball_size * config.region_size)).ceil() as usize;
                particle_system.instances.regions_y =
                    (1.0 / (0.16 * config.ball_size * config.region_size)).ceil() as usize;

                if particle_system.instances.regions_x == 0
                    || particle_system.instances.regions_y == 0
                {
                    return;
                }

                particle_system.instances.bounding_box =
                    particle_system.particle_system_data.domain;
                particle_system.instances.rebuild_regions();

                for x in 0..particle_system.instances.regions_x {
                    for y in 0..particle_system.instances.regions_y {
                        for a in 0..particle_system.instances.get_region_mut(x, y).len() {
                            let i = particle_system.instances.get_region_mut(x, y)[a];
                            let mut density = 0;
                            let instance = particle_system.instances[i];
                            let mut velocity_if_correcting_it = 0.0;

                            if config.correct_ball_velocity {
                                velocity_if_correcting_it =
                                    particle_system.particle_data[i].velocity.magnitude();

                                if velocity_if_correcting_it.is_normal() {
                                    let scalar = (config.ball_speed / self.actual_ball_speed - 1.0)
                                        * dt.as_secs_f32()
                                        / 10.0
                                        + 1.0;
                                    particle_system.particle_data[i]
                                        .velocity
                                        .mul_assign(scalar.clamp(0.5, 2.0));

                                    if particle_system.particle_data[i].velocity.magnitude2()
                                        > (config.ball_speed * config.ball_speed * 1000.0).max(10.0)
                                    // we don't want false detections with low configured ball speeds
                                    {
                                        log::error!("Particle velocity went haywire. Resetting it to a new random velocity. (velocity: {}, before correcting: {}, scalar: {})", particle_system.particle_data[i].velocity.magnitude(), velocity_if_correcting_it, scalar);
                                        let mut move_vector = Vector3::new(
                                            random::<f32>() - 0.5,
                                            random::<f32>() - 0.5,
                                            //random::<f32>() - 0.5,
                                            0.0,
                                        );
                                        move_vector = move_vector.normalize() * config.ball_speed;
                                        particle_system.particle_data[i].velocity = move_vector;
                                    }
                                    total_velocity += velocity_if_correcting_it;
                                } else {
                                    log::error!("Velocity is not normal. Resetting it to new random velocity. (velocity: {:?}, index: {})", particle_system.particle_data[i].velocity, i);
                                    let mut move_vector = Vector3::new(
                                        random::<f32>() - 0.5,
                                        random::<f32>() - 0.5,
                                        //random::<f32>() - 0.5,
                                        0.0,
                                    );
                                    move_vector = move_vector.normalize() * config.ball_speed;
                                    particle_system.particle_data[i].velocity = move_vector;
                                }
                            }

                            //particle_system.particle_data[i].velocity.add_assign(GRAVITY.mul(dt.as_secs_f32()));

                            particle_system
                                .instances
                                .get_regions_in_range(x, y, 1)
                                .iter()
                                .for_each(|&j| {
                                    density += 1;
                                    if i > j {
                                        let other_instance = particle_system.instances[j];
                                        let other_data = particle_system.particle_data[j];
                                        let data = particle_system.particle_data[i];

                                        //check if the bals collide
                                        if (instance.position.x - other_instance.position.x)
                                            * (instance.position.x - other_instance.position.x)
                                            + (instance.position.y - other_instance.position.y)
                                                * (instance.position.y - other_instance.position.y)
                                            < instance.scale
                                                * data.collider.unwrap().x
                                                * instance.scale
                                                * data.collider.unwrap().y
                                        {
                                            let distance =
                                                instance.position.distance(other_instance.position);
                                            let target_distance =
                                                config.ball_size * data.collider.unwrap().x;

                                            let n = (instance.position - other_instance.position)
                                                .normalize();
                                            particle_system.instances[i]
                                                .position
                                                .add_assign(n * (target_distance - distance) / 2.0);
                                            particle_system.instances[j].position.add_assign(
                                                -n * (target_distance - distance) / 2.0,
                                            );
                                            let v1 = -data.velocity;
                                            let v2 = -other_data.velocity;
                                            let c1 = instance.position;
                                            let c2 = other_instance.position;

                                            //https://stackoverflow.com/questions/35211114/2d-elastic-ball-collision-physics
                                            particle_system.particle_data[i].velocity = -v1
                                                + (c1 - c2) * (v1 - v2).dot(c1 - c2)
                                                    / (c1 - c2).magnitude2();
                                            particle_system.particle_data[j].velocity = -v2
                                                + (c2 - c1) * (v2 - v1).dot(c2 - c1)
                                                    / (c2 - c1).magnitude2();

                                            match config.color_mode {
                                                BallColorMode::Random => {
                                                    let col = util::random_color();

                                                    particle_system.instances[i].color = col;
                                                    particle_system.instances[j].color = col;
                                                }
                                                BallColorMode::Infection => {
                                                    if (util::compare_colors_ignoring_alpha(
                                                        other_instance.color,
                                                        self.color,
                                                    ) || util::compare_colors_ignoring_alpha(
                                                        instance.color,
                                                        self.color,
                                                    )) && !util::compare_colors_ignoring_alpha(
                                                        instance.color,
                                                        other_instance.color,
                                                    ) {
                                                        particle_system.instances[i].color =
                                                            self.color;
                                                        particle_system.instances[j].color =
                                                            self.color;
                                                    }
                                                }
                                                _ => {}
                                            }
                                            //particle_system.instances[i].age = Duration::new(0, 0);
                                        }
                                    }
                                });

                            match config.color_mode {
                                BallColorMode::Temperature => {
                                    let hsv = Hsv::new(
                                        angular_units::Turns(
                                            (((if config.correct_ball_velocity {
                                                velocity_if_correcting_it
                                            } else {
                                                particle_system.particle_data[i]
                                                    .velocity
                                                    .magnitude()
                                            }) / config.ball_speed
                                                - 0.5)
                                                .max(0.0)
                                                / 50.0)
                                                .clamp(0.0, 0.9),
                                        ),
                                        1.0,
                                        1.0,
                                    );
                                    let rgb = Rgb::from(hsv);
                                    particle_system.instances[i].color = Color {
                                        r: rgb.red(),
                                        g: rgb.green(),
                                        b: rgb.blue(),
                                        a: 1.0,
                                    }
                                }
                                BallColorMode::Infection => {
                                    if util::compare_colors_ignoring_alpha(
                                        instance.color,
                                        self.color,
                                    ) {
                                        infected_balls += 1;
                                    }
                                }
                                _ => {}
                            }
                            if config.show_density {
                                let density = f64::clamp(
                                    density as f64 / config.target_display_density,
                                    0.0,
                                    1.0,
                                );
                                particle_system.instances()[i].color.a = density * density;
                            }
                        }
                    }
                }

                if infected_balls >= config.ball_count {
                    self.color = util::random_color();
                    particle_system
                        .instances
                        .instances
                        .choose_mut(&mut rand::thread_rng())
                        .unwrap()
                        .color = self.color;
                }

                particle_system.update_instance_buffer(queue);
            };
            model.update(dt, queue);
        }

        self.actual_ball_speed = total_velocity / config.ball_count as f32;
        /*
        println!(
            "a/average velocity: {},\ttarget: {}",
            total_velocity / self.ball_count as f32,
            self.ball_speed
        )*/
    }

    fn resize(&mut self, old_ratio: f32, new_ratio: f32) {
        for model in &mut self.balls {
            //get (ParticleSystem)(Object) idiot
            if let Some(particle_system) = model.mesh.as_any_mut().downcast_mut::<ParticleSystem>()
            {
                for instance in particle_system.instances.iter_mut() {
                    instance.position.x *= new_ratio / old_ratio;
                }
                particle_system.particle_system_data.domain = BoundingBox::new_with_size(
                    //Vector3::new(0.0, 0.0, 0.55),
                    Vector3::new(0.0, 0.0, 0.0),
                    2.0 * new_ratio,
                    2.0,
                    //1.00,
                    0.0,
                    BoundingBoxType::Bounce,
                );
            }
        }
    }

    fn get_background_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }
    }

    fn handle_input(&mut self, position: [f32; 2], id: u64, enabled: bool) -> bool {
        let mut id = id;
        //dumb chromium doing touch IDs in a dumb way
        if id > 0 {
            id = (id - 1) % 5 + 1
        }

        if !self.first_input_handled {
            self.first_input_handled = true;
            return false;
        }
        if id > 5 {
            return false;
        }
        if !enabled {
            self.inputs[id as usize] = None;
            return false;
        }

        let old_input = self.inputs[id as usize];
        const BRUSH_SIZE: f32 = 0.15;
        if let Some(old_input) = old_input {
            for model in &mut self.balls {
                //get (ParticleSystem)(Object) idiot
                if let Some(particle_system) =
                    model.mesh.as_any_mut().downcast_mut::<ParticleSystem>()
                {
                    let x: f32 = position[0] / 2.0 + 0.5;
                    let y: f32 = position[1] / 2.0 + 0.5;
                    for i in particle_system.instances.get_regions_in_range(
                        usize::clamp(
                            (x * particle_system.instances.regions_x as f32) as usize,
                            0,
                            particle_system.instances.regions_x - 1,
                        ),
                        usize::clamp(
                            (y * particle_system.instances.regions_y as f32) as usize,
                            0,
                            particle_system.instances.regions_y - 1,
                        ),
                        (particle_system.instances.regions_y as f32 / 2.0 * BRUSH_SIZE).ceil()
                            as u32,
                    ) {
                        //if particle_system.instances[i].position.distance2(Vector3::new(position[0], position[1], 0.0)) < BRUSH_SIZE * BRUSH_SIZE {
                        particle_system.particle_data[i]
                            .velocity
                            .add_assign(Vector3::new(
                                position[0] - old_input[0],
                                position[1] - old_input[1],
                                0.0,
                            ));
                        //}
                    }
                }
            }
        }
        self.inputs[id as usize] = Some(position);
        false
    }

    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, state: &State<'_>) {
        // lib.rmesh.in
        render_pass.set_pipeline(&state.render_pipeline);
        render_pass.set_bind_group(1, &state.camera_bind_group, &[]);

        for model in &self.balls {
            render_pass.set_bind_group(0, &model.material.bind_group, &[]);
            render_pass.draw_mesh_instanced(&*model.mesh, 0..model.mesh.instance_count() as u32);
        }
    }
}

pub struct SnowScreenSaver {
    pub(crate) models: Vec<Model>,
    old_config: Configurator,
}

impl ScreenSaver for SnowScreenSaver {
    fn new(config: Configurator) -> SnowScreenSaver
    where
        Self: Sized,
    {
        Self {
            models: vec![],
            old_config: config,
        }
    }

    fn setup(
        &mut self,
        _size: Size,
        config: &Configurator,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) {
        //ground defined first so it gets drawn first and doesn't get occluded by the snow
        let ground1 = include_bytes!("resources/textures/ground1.png");
        let diffuse_texture =
            texture::Texture::from_bytes(device, queue, ground1, "ground1.png").unwrap();
        let billboard = util::create_billboard(
            6.0,
            3.0,
            Vector3::new(0.0, 0.0, 0.1),
            diffuse_texture,
            &device,
            &layout,
        )
        .unwrap();
        self.models.push(billboard);

        let ground2 = include_bytes!("resources/textures/ground2.png");
        let diffuse_texture =
            texture::Texture::from_bytes(device, queue, ground2, "ground2.png").unwrap();
        let billboard = util::create_billboard(
            6.0,
            3.0,
            Vector3::new(0.0, 0.0, 0.3),
            diffuse_texture,
            &device,
            &layout,
        )
        .unwrap();
        self.models.push(billboard);

        let ground3 = include_bytes!("resources/textures/ground3.png");
        let diffuse_texture =
            texture::Texture::from_bytes(device, queue, ground3, "ground3.png").unwrap();
        let billboard = util::create_billboard(
            6.0,
            3.0,
            Vector3::new(0.0, 0.0, 0.5),
            diffuse_texture,
            &device,
            &layout,
        )
        .unwrap();
        self.models.push(billboard);

        let snow1 = include_bytes!("resources/textures/snow1.png");
        let snow2 = include_bytes!("resources/textures/snow2.png");
        let diffuse_textures = [
            texture::Texture::from_bytes(device, queue, snow1, "snow1.png").unwrap(),
            texture::Texture::from_bytes(device, queue, snow2, "snow2.png").unwrap(),
        ];
        for diffuse_texture in diffuse_textures {
            let mut snow_particle_system = ParticleSystem::create_billboard(
                0.03,
                0.03,
                Vector3::new(0.0, 0.0, 0.0),
                ParticleSystemData::new(BoundingBox::new_with_size(
                    Vector3::new(0.0, 0.0, 0.5),
                    6.0,
                    3.0,
                    1.0,
                    BoundingBoxType::Modulo,
                )),
                device,
            );

            let snow_material = Material::new(diffuse_texture, device, layout);

            snow_particle_system.populate_random(config.snowflake_count, device);
            for i in 0..snow_particle_system.instances.len() {
                let particle = &mut snow_particle_system.instances[i];
                let data = &mut snow_particle_system.particle_data[i];
                particle.position.z = 1.0 - particle.position.z * particle.position.z;
                particle.scale = 1.0 - particle.position.z * 0.8;
                particle.color.a = 1.0 - particle.position.z as f64;
                data.velocity = Vector3::new(
                    (random::<f32>() * 0.1 - 0.4) * particle.scale,
                    (random::<f32>() * 0.1 + 0.5) * particle.scale,
                    0.0,
                )
            }

            let snow = Model {
                mesh: Box::new(snow_particle_system),
                material: snow_material,
            };

            self.models.push(snow);
        }

        /*
        let moon = include_bytes!("resources/textures/moon.png");
        let diffuse_texture =
            texture::Texture::from_bytes(&device, &queue, moon, "moon.png").unwrap();
        let billboard = resource::create_billboard(
            0.32,
            0.32,
            Vector3::new(-1.0, -1.0, 1.0),
            diffuse_texture,
            &device,
            &layout,
        )
            .unwrap();
        self.models.push(billboard);*/
    }

    fn update(
        &mut self,
        _size: winit::dpi::Size,
        config: &Configurator,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        dt: Duration,
    ) {
        if self.old_config != *config {
            for model in &mut self.models {
                //get (ParticleSystem)(Object) idiot
                if let Some(particle_system) =
                    model.mesh.as_any_mut().downcast_mut::<ParticleSystem>()
                {
                    if config.snowflake_count != self.old_config.snowflake_count {
                        let delta =
                            config.snowflake_count as i32 - self.old_config.snowflake_count as i32;
                        if delta > 0 {
                            particle_system.populate_random(delta.try_into().unwrap(), device);
                            for i in
                                self.old_config.snowflake_count..particle_system.instances.len()
                            {
                                let particle = &mut particle_system.instances.instances[i];
                                let data = &mut particle_system.particle_data[i];
                                particle.position.z =
                                    1.0 - particle.position.z * particle.position.z;
                                particle.scale = 1.0 - particle.position.z * 0.8;
                                particle.color.a = 1.0 - particle.position.z as f64;
                                data.velocity = Vector3::new(
                                    (random::<f32>() * 0.1 - 0.4) * particle.scale,
                                    (random::<f32>() * 0.1 + 0.5) * particle.scale,
                                    0.0,
                                )
                            }
                        } else {
                            particle_system
                                .instances
                                .instances
                                .truncate(config.snowflake_count);
                            particle_system
                                .particle_data
                                .truncate(config.snowflake_count);
                        }

                        model.mesh.rebuild_instance_buffer(device);
                    }
                }
            }
            self.old_config = *config;
        }

        for model in &mut self.models {
            model.update(dt, queue);
        }
    }

    fn resize(&mut self, _old_ratio: f32, _new_ratio: f32) {
        //no need to do nothin'
    }

    fn get_background_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: 0.0,
            g: 0.0,
            b: 0.01,
            a: 1.0,
        }
    }

    fn handle_input(&mut self, _position: [f32; 2], _id: u64, _enabled: bool) -> bool {
        false
    }

    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, state: &State<'_>) {
        // lib.rmesh.in
        render_pass.set_pipeline(&state.render_pipeline);
        render_pass.set_bind_group(1, &state.camera_bind_group, &[]);

        for model in &self.models {
            render_pass.set_bind_group(0, &model.material.bind_group, &[]);
            render_pass.draw_mesh_instanced(&*model.mesh, 0..model.mesh.instance_count() as u32);
        }
    }
}
