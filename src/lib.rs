pub mod configurator;
mod instance;
mod model;
mod particle;
mod screensaver;
mod shaders;
mod texture;
mod util;

use winit::event::KeyEvent;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};
use wgpu::BindGroupLayout;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowBuilderExtWebSys;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

use crate::instance::LayoutDescriptor;
use crate::screensaver::{ScreenSaver, ScreenSaverType};
use cgmath::prelude::*;
use cgmath::Matrix4;
use config::{Config, FileFormat};
use model::Vertex;
use std::collections::HashSet;
use std::process;
use std::sync::{Arc, Mutex};
use wgpu::util::DeviceExt;
use wgpu::Limits;
use winit::dpi::Size;
use winit::error::EventLoopError;
//#[cfg(debug_assertions)]
//#[cfg(not(target_arch = "wasm32"))]
//use winit::event::KeyEvent;
use winit::event::{ElementState, Event, TouchPhase, WindowEvent};
#[cfg(target_arch = "wasm32")]
use winit::event::{MouseButton};

use crate::configurator::{ConfigUI, Configurator};
use crate::model::ModelInstanceRaw;
use particle::ParticleInstanceRaw;
use util::render;
use winit::event_loop::{EventLoop, EventLoopBuilder};
use winit::keyboard::{Key, NamedKey};
#[cfg(target_os = "windows")]
use winit::platform::windows::EventLoopBuilderExtWindows;
#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;
use winit::window::{Fullscreen, Window, WindowBuilder};

pub const DEFAULT_CONFIG: &[u8] = include_bytes!("resources/default_config.toml");

#[cfg(target_arch = "wasm32")]
pub fn get_config() -> Config {
    //yes I am converting request parameters into a .toml file and passing it as a config what about it
    let url_params = web_sys::UrlSearchParams::new_with_str(
        web_sys::window()
            .unwrap()
            .location()
            .search()
            .unwrap()
            .as_str(),
    )
    .unwrap();
    let mut params_toml = String::new();
    for param in url_params.keys() {
        //log::error!("{}", param.unwrap().as_string().unwrap());
        let key_str = param.clone().unwrap().as_string().unwrap();
        params_toml.push_str(&*format!(
            "{} = \"{}\"\n",
            key_str,
            url_params.get(key_str.as_str()).unwrap()
        ));
        if key_str == "screensaver" {
            params_toml.push_str(&*format!(
                "[{}]\n",
                url_params.get(key_str.as_str()).unwrap()
            ));
        }
    }

    Config::builder()
        .add_source(config::File::from_str(
            std::str::from_utf8(DEFAULT_CONFIG).expect("Failed to read the default config"),
            FileFormat::Toml,
        ))
        .add_source(config::File::from_str(&*params_toml, FileFormat::Toml))
        .build()
        .unwrap()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_config() -> Config {
    let mut config_path = dirs::config_dir().unwrap().to_path_buf();
    config_path.push("michaels-screensaver.toml");

    Config::builder()
        .add_source(config::File::from_str(
            std::str::from_utf8(DEFAULT_CONFIG).expect("Failed to read the default config"),
            FileFormat::Toml,
        ))
        .add_source(config::File::with_name(config_path.to_str().unwrap()))
        .add_source(config::Environment::with_prefix("APP"))
        .build()
        .unwrap()
}

#[cfg(target_arch = "wasm32")]
pub fn get_default_config() -> Config {
    Config::builder()
        .add_source(config::File::from_str(
            std::str::from_utf8(DEFAULT_CONFIG).expect("Failed to read the default config"),
            FileFormat::Toml,
        ))
        .build()
        .unwrap()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_default_config() -> Config {
    Config::builder()
        .add_source(config::File::from_str(
            std::str::from_utf8(DEFAULT_CONFIG).expect("Failed to read the default config"),
            FileFormat::Toml,
        ))
        .add_source(config::Environment::with_prefix("APP"))
        .build()
        .unwrap()
}

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    // We can't use cgmath with bytemuck directly, so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

#[allow(dead_code)]
enum CameraType {
    Perspective(f32),
    Orthographic(),
}

struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    znear: f32,
    zfar: f32,
    ratio: f32,
    camera_type: CameraType,
}

struct CameraController {
    pressed_keys: HashSet<Key>,
}

impl CameraController {
    fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
        }
    }

    fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            //#[cfg(debug_assertions)]
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    state, logical_key, ..
                },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                if is_pressed {
                    self.pressed_keys.insert(logical_key.clone());
                } else {
                    self.pressed_keys.remove(logical_key);
                }
                false
            }
            _ => false,
        }
    }

    fn update_camera(&self, camera: &mut Camera) {
        let move_delta = 0.1;

        for key in self.pressed_keys.iter() {
            if let Key::Character(char) = key {
                match camera.camera_type {
                    CameraType::Orthographic() => match char.to_ascii_lowercase().as_str() {
                        "w" => camera.eye.y -= move_delta,
                        "s" => camera.eye.y += move_delta,
                        "d" => camera.eye.x -= move_delta,
                        "a" => camera.eye.x += move_delta,
                        _ => {}
                    },
                    CameraType::Perspective(_) => match char.to_ascii_lowercase().as_str() {
                        "s" => camera.eye.z += move_delta,
                        "w" => camera.eye.z -= move_delta,
                        "e" => camera.eye.y += move_delta,
                        "q" => camera.eye.y -= move_delta,
                        "a" => camera.eye.x -= move_delta,
                        "d" => camera.eye.x += move_delta,
                        _ => {}
                    },
                }
            }
        }
    }
}

#[rustfmt::skip]
impl Camera {
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let (view, proj) = match self.camera_type {
            CameraType::Perspective(fov) => (
                cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up),
                cgmath::perspective(cgmath::Deg(fov), self.ratio, self.znear, self.zfar),
            ),
            CameraType::Orthographic() => {
                (
                    cgmath::Matrix4::from_translation(self.eye.to_vec()),
                    //cgmath::ortho(self.ratio/2.0, -self.ratio/2.0, 0.5, -0.5,  1.0, 0.0)

                    //custom orthographic projection matrix
                    Matrix4::new(
                        -1.0 / self.ratio, 0.0, 0.0, 0.0, //x
                        0.0, -1.0, 0.0, 0.0, //y
                        self.eye.x - self.target.x, self.eye.y - self.target.y, 0.1, 0.0, //z
                        0.0, 0.0, 0.0, 1.0, //w
                    ), /*
                       Matrix4::new(
                           1.0/self.ratio, 0.0, 0.0, 0.0, //x
                           0.0, 1.0, 0.0, 0.0, //y
                           0.0, 0.0, 1.0, 0.0, //z
                           0.0, 0.0, 0.0, 1.0, //w
                       )*/
                )
            }
        };

        OPENGL_TO_WGPU_MATRIX * proj * view
    }
}

struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: &'a Window,
    background_color: wgpu::Color,
    camera: Camera,
    camera_controller: CameraController,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth_texture: texture::Texture,
    screensaver: Box<dyn ScreenSaver>,
    screensaver_type: ScreenSaverType,
    last_updated: Instant,
    texture_bind_group_layout: BindGroupLayout,
    render_pipeline_layout: wgpu::PipelineLayout,
}

impl<'a> State<'a> {
    // Creating some of the wgpu types requires async code
    async fn new(window: &'a Window, configurator: &Configurator) -> State<'a> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        /*
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::GL,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });
        */
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(), //fuck it anything will do
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower, //we don't need the highest performance for a screen saver
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await;

        match adapter {
            Some(adapter) => {
                let (device, queue) = adapter
                    .request_device(
                        &wgpu::DeviceDescriptor {
                            required_features: wgpu::Features::empty(),
                            // WebGL doesn't support all of wgpu's features, so if
                            // we're building for the web, we'll have to disable some.
                            required_limits: if cfg!(target_arch = "wasm32") {
                                //wgpu::Limits::downlevel_webgl2_defaults()
                                Limits::downlevel_webgl2_defaults()
                                    .using_resolution(adapter.limits())
                            } else {
                                wgpu::Limits::default()
                            },
                            label: None,
                            memory_hints: Default::default(),
                        },
                        None, // Trace path
                    )
                    .await
                    .unwrap();

                let surface_caps = surface.get_capabilities(&adapter);
                // Shader code in this tutorial assumes an sRGB surface texture. Using a different
                // one will result in all the colors coming out darker. If you want to support non
                // sRGB surfaces, you'll need to account for that when drawing to the frame.
                let surface_format = surface_caps
                    .formats
                    .iter()
                    .find(|f| f.is_srgb())
                    .copied()
                    .unwrap_or(surface_caps.formats[0]);
                let config = wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT, /*| wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST*/
                    format: surface_format,
                    width: size.width.max(1),
                    height: size.height.max(1),
                    //present_mode: surface_caps.present_modes[0],
                    present_mode: wgpu::PresentMode::AutoVsync,
                    alpha_mode: surface_caps.alpha_modes[0],
                    view_formats: vec![],
                    desired_maximum_frame_latency: 2,
                };

                surface.configure(&device, &config);

                let texture_bind_group_layout =
                    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    multisampled: false,
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                // This should match the filterable field of the
                                // corresponding Texture entry above.
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                        ],
                        label: Some("texture_bind_group_layout"),
                    });

                let screensaver_type = &configurator.screensaver;

                let mut screensaver: Box<dyn ScreenSaver> = match screensaver_type {
                    ScreenSaverType::Snow => {
                        Box::new(screensaver::SnowScreenSaver::new(*configurator))
                    }
                    ScreenSaverType::Balls => {
                        Box::new(screensaver::BallScreenSaver::new(*configurator))
                    }
                    ScreenSaverType::DDDModel => {
                        Box::new(screensaver::DDDModelScreensaver::new(*configurator))
                    }
                };

                let campos = screensaver.get_camera_position();

                let camera = Camera {
                    eye: campos.0,
                    target: campos.1,
                    up: cgmath::Vector3::unit_y(),
                    znear: 0.1,
                    zfar: 100.0,
                    ratio: config.width as f32 / config.height as f32,
                    camera_type: screensaver.get_camera_type(),
                };

                let camera_controller = CameraController::new();

                let camera_bind_group_layout =
                    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        entries: &[wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        }],
                        label: Some("camera_bind_group_layout"),
                    });

                let mut camera_uniform = CameraUniform::new();
                camera_uniform.update_view_proj(&camera);

                let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Camera Buffer"),
                    contents: bytemuck::cast_slice(&[camera_uniform]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

                let background_color = wgpu::Color {
                    r: 0.1,
                    g: 0.1,
                    b: 0.1,
                    a: 1.0,
                };

                let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &camera_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera_buffer.as_entire_binding(),
                    }],
                    label: Some("camera_bind_group"),
                });

                let depth_texture =
                    texture::Texture::create_depth_texture(&device, &config, "depth_texture");

                let render_pipeline_layout =
                    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Render Pipeline Layout"),
                        bind_group_layouts: &[
                            &texture_bind_group_layout,
                            &camera_bind_group_layout,
                        ],
                        push_constant_ranges: &[],
                    });


                screensaver.setup(
                    Size::from(size),
                    configurator,
                    &device,
                    &queue,
                    &texture_bind_group_layout,
                    &render_pipeline_layout,
                    config.format,
                    Some(texture::Texture::DEPTH_FORMAT),
                );

                Self {
                    window,
                    surface,
                    device,
                    queue,
                    config,
                    size,
                    background_color,
                    depth_texture,
                    camera,
                    camera_controller,
                    camera_uniform,
                    camera_buffer,
                    camera_bind_group,
                    texture_bind_group_layout,
                    render_pipeline_layout,
                    screensaver,
                    screensaver_type: *screensaver_type,
                    last_updated: Instant::now(),
                }
            }
            None => {
                panic!("Unable to find an appropriate graphics adapter");
            }
        }
    }

    pub fn window(&self) -> &Window {
        self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
        self.depth_texture =
            texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        self.screensaver.resize(
            self.camera.ratio,
            new_size.width as f32 / new_size.height as f32,
        );
        self.camera.ratio = new_size.width as f32 / new_size.height as f32;
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        if /* !self.camera_controller.process_events(event)*/ true {
            match event {
                WindowEvent::CursorMoved { position, .. } => self.screensaver.handle_input(
                    [
                        -(position.x as f32 / self.size.width as f32) * 2.0 + 1.0,
                        (position.y as f32 / self.size.height as f32) * 2.0 - 1.0,
                    ],
                    0,
                    true,
                ),
                WindowEvent::Touch(touch) => {
                    let position = touch.location;
                    self.screensaver.handle_input(
                        [
                            -(position.x as f32 / self.size.width as f32) * 2.0 + 1.0,
                            (position.y as f32 / self.size.height as f32) * 2.0 - 1.0,
                        ],
                        touch.id + 1,
                        matches!(touch.phase, TouchPhase::Started | TouchPhase::Moved),
                    )
                }
                _ => false,
            }
        } else {
            true
        }
    }

    fn update(&mut self, config: &mut Configurator) {
        self.camera_controller.update_camera(&mut self.camera);
        let cam_pos = self.screensaver.get_camera_position();
        self.camera.eye = cam_pos.0;
        self.camera.target = cam_pos.1;

        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
        let last_updated = Instant::now();
        if self.screensaver_type != config.screensaver || config.should_reload {
            config.should_reload = false;
            self.screensaver = match config.screensaver {
                ScreenSaverType::Snow => Box::new(screensaver::SnowScreenSaver::new(*config)),
                ScreenSaverType::Balls => Box::new(screensaver::BallScreenSaver::new(*config)),
                ScreenSaverType::DDDModel => {
                    Box::new(screensaver::DDDModelScreensaver::new(*config))
                }
            };
            self.screensaver_type = config.screensaver;

            self.screensaver.setup(
                Size::from(self.size),
                config,
                &self.device,
                &self.queue,
                &self.texture_bind_group_layout,
                &self.render_pipeline_layout,
                self.config.format,
                Some(texture::Texture::DEPTH_FORMAT),
            );

            self.camera.camera_type = self.screensaver.get_camera_type();
        }
        self.screensaver.update(
            Size::from(self.size),
            config,
            &self.device,
            &self.queue,
            Instant::now().duration_since(self.last_updated),
        );
        self.background_color = self.screensaver.get_background_color();

        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let mut size_x = web_sys::window().unwrap().inner_width().unwrap().as_f64().unwrap();
                let mut size_y = web_sys::window().unwrap().inner_height().unwrap().as_f64().unwrap();

            let config_canvas = web_sys::window().unwrap().document().unwrap()
                .get_element_by_id("config_canvas")
                .expect("Failed to find config_canvas")
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .expect("config_canvas was not a HtmlCanvasElement");

                let scale = web_sys::window().unwrap().device_pixel_ratio();

                config_canvas.set_width((250.0 * web_sys::window().unwrap().device_pixel_ratio()) as u32);
                config_canvas.set_height((300.0 * web_sys::window().unwrap().device_pixel_ratio()) as u32);
                //config_canvas.set_width(300);
                //config_canvas.set_height(300);

                //config_canvas.style().set_property("transform", &*format! {"scale({})", 1.0/scale}).unwrap();

                let  _ = self.window.request_inner_size(winit::dpi::LogicalSize::new(size_x, size_y));

                /*
                let _ = self.window.canvas().unwrap().style().set_property(
                    "transform",
                    &*format! {"scale({})", scale},
                );*/

                if self.window.fullscreen().is_some() {
                    self.window.set_cursor_visible(false);
                } else {
                    self.window.set_cursor_visible(true);
                }
            }
        }
        self.last_updated = last_updated;
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.background_color),
                        //load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            self.screensaver.render(&mut render_pass, self);
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub async fn run() {
    let configurator = Configurator::from_config(get_config());
    let configurator = Arc::new(Mutex::new(configurator));
    run_with_config(configurator).await;
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[cfg(target_arch = "wasm32")]
pub async fn run_with_config_window() {
    let configurator = Configurator::from_config(get_config());
    let configurator = Arc::new(Mutex::new(configurator));
    let config_ui = ConfigUI::new(Arc::clone(&configurator));

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        run_with_config(configurator).await;
    });

    let document = web_sys::window()
        .expect("No window")
        .document()
        .expect("No document");

    let canvas = document
        .get_element_by_id("config_canvas")
        .expect("Failed to find config_canvas")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("config_canvas was not a HtmlCanvasElement");

    let _start_result = eframe::WebRunner::new()
        .start(canvas, web_options, Box::new(|cc| Ok(Box::new(config_ui))))
        .await;
}

pub async fn run_with_config(configurator: Arc<Mutex<Configurator>>) {
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
    }
    {
        log::info!("Starting main loop");
        #[cfg(target_arch = "wasm32")]
        let event_loop: Result<EventLoop<()>, EventLoopError> = EventLoopBuilder::default().build();
        #[cfg(not(target_arch = "wasm32"))]
        let event_loop: Result<EventLoop<()>, EventLoopError> =
            EventLoopBuilder::default().with_any_thread(true).build();

        match event_loop {
            Ok(event_loop) => {
                let window = match configurator.lock() {
                    Ok(configurator) => {
                        cfg_if::cfg_if! {
                            if #[cfg(target_arch = "wasm32")] {
                                let canvas = web_sys::window().unwrap().document().unwrap().get_element_by_id("screensaver").unwrap()
                                    .dyn_into::<web_sys::HtmlCanvasElement>()
                                    .map_err(|_| ())
                                    .unwrap();
                                WindowBuilder::new()
                                    .with_canvas(Some(canvas))
                                    .build(&event_loop).unwrap()
                                }
                            else {
                                if configurator.fullscreen && !configurator.preview_window {
                                    WindowBuilder::new()
                                    .with_fullscreen(Some(Fullscreen::Borderless(None)))
                                    .build(&event_loop).unwrap()
                                        }
                                else {
                                    WindowBuilder::new()
                                        .build(&event_loop).unwrap()
                                }
                                //window.set_cursor_visible(false);
                            }
                        }
                    }
                    Err(e) => panic!("failed to lock configurator: {}", e),
                };

                let mut state = match configurator.lock() {
                    Ok(configurator) => State::new(&window, &configurator).await,
                    Err(e) => panic!("failed to lock configurator: {}", e),
                };

                let result = event_loop.run(|event, control_flow| {
                    if let Ok(mut configurator) = configurator.lock() {
                        if let Event::WindowEvent {
                            ref event,
                            window_id,
                        } = event
                        {
                            if !state.input(event) && window_id == state.window().id() {
                                match event {
                                    #[cfg(not(target_arch = "wasm32"))]
                                    WindowEvent::CloseRequested => {
                                        if !configurator.preview_window {
                                            control_flow.exit();
                                            process::exit(0);
                                        }
                                    }
                                    #[cfg(not(target_arch = "wasm32"))]
                                    WindowEvent::MouseInput {
                                        state: ElementState::Pressed,
                                        ..
                                    } => {
                                        if configurator.fullscreen && !configurator.preview_window {
                                            control_flow.exit();
                                            process::exit(0);
                                        }
                                    }
                                    //#[cfg(not(debug_assertions))]
                                    #[cfg(not(target_arch = "wasm32"))]
                                    WindowEvent::KeyboardInput {
                                        event,
                                        is_synthetic: false,
                                        ..
                                    } => {
                                        //exit the screensaver when any key is pressed, but not on the web (duh)
                                        log::debug!("{:?}", event);

                                        if event.state == ElementState::Pressed {
                                            //stupid windows sending a stupid random key event at the start of the program
                                            if cfg!(target_os = "windows") {
                                                match event.logical_key {
                                                    Key::Named(NamedKey::AltGraph) => {}
                                                    _ => control_flow.exit(),
                                                }
                                            } else if configurator.fullscreen
                                                && !configurator.preview_window
                                            {
                                                control_flow.exit();
                                                process::exit(0);
                                            }
                                        }
                                    }
                                    #[cfg(target_arch = "wasm32")]
                                    WindowEvent::KeyboardInput {
                                        event:
                                            KeyEvent {
                                                state: ElementState::Pressed,
                                                logical_key,
                                                ..
                                            },
                                        ..
                                    } => match logical_key {
                                        Key::Named(NamedKey::Escape) => {
                                            state.window.set_fullscreen(None);
                                        }
                                        Key::Named(NamedKey::F11) => {
                                            state
                                                .window
                                                .set_fullscreen(Some(Fullscreen::Borderless(None)));
                                        }
                                        Key::Character(char) if char == "f" => {
                                            state
                                                .window
                                                .set_fullscreen(Some(Fullscreen::Borderless(None)));
                                        }
                                        _ => {}
                                    },
                                    #[cfg(target_arch = "wasm32")]
                                    WindowEvent::MouseInput {
                                        button: MouseButton::Left,
                                        state: ElementState::Pressed,
                                        ..
                                    }
                                    | WindowEvent::Touch(..) => {
                                        state
                                            .window
                                            .set_fullscreen(Some(Fullscreen::Borderless(None)));
                                    }

                                    WindowEvent::Resized(physical_size) => {
                                        state.resize(*physical_size);
                                    }
                                    WindowEvent::RedrawRequested => {
                                        state.window().request_redraw();
                                        state.window().set_visible(true);

                                        /*
                                        if !surface_configured {
                                            return;
                                        }*/

                                        state.update(&mut configurator);
                                        match state.render() {
                                            Ok(_) => {}
                                            // Reconfigure the surface if it's lost or outdated
                                            Err(
                                                wgpu::SurfaceError::Lost
                                                | wgpu::SurfaceError::Outdated,
                                            ) => state.resize(window.inner_size()),
                                            // The system is out of memory, we should probably quit
                                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                                log::error!("Out Of Memory");
                                                control_flow.exit();
                                            }

                                            // This happens when the a frame takes too long to present
                                            Err(wgpu::SurfaceError::Timeout) => {
                                                log::warn!("Surface timeout")
                                            }
                                            Err(wgpu::SurfaceError::Other) => {
                                                log::error!("Other render error ¯\\_(ツ)_/¯")
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                });

                match result {
                    Ok(_) => {
                        log::info!("Window closed without errors");
                    }
                    Err(err) => {
                        log::error!("Window closed with error: {:?}", err);
                    }
                }
            }
            Err(err) => log::error!("Failed to create the event loop: {:?}", err),
        }
    }
}
