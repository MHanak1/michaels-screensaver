#![windows_subsystem = "windows"]

mod model;
mod resource;
mod screensaver;
mod shaders;
mod texture;

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowBuilderExtWebSys;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

use crate::screensaver::{ScreenSaver, ScreenSaverType};
use cgmath::prelude::*;
use cgmath::{Matrix4, Vector3};
use image::GenericImageView;
use model::Vertex;
use std::collections::HashSet;
use std::ops::Add;
use wgpu::util::DeviceExt;
use winit::error::EventLoopError;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey, SmolStr};
use winit::window::{Fullscreen, Window, WindowAttributes, WindowBuilder};

//TODO: implement scale
struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
    color: wgpu::Color,
    velocity: cgmath::Vector3<f32>, //in case i wanted to update the position through a compute shader (don't know how yet)
    scale: f32,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position)
                * cgmath::Matrix4::from(self.rotation))
            .into(),
            //model: Matrix4::from_translation(Vector3::zero()).into(),
            color: [self.color.r as f32, self.color.g as f32, self.color.b as f32, self.color.a as f32],
            //velocity: self.velocity.into(),
            scale: self.scale,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
    color: [f32; 4],
    //velocity: [f32; 3],
    scale: f32,
}

impl InstanceRaw {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in the shader.
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials, we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5, not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 20]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
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

const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    -1.0, 0.0, 0.0, 0.0,
    0.0, -1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

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
    speed: f32,
    pressed_keys: HashSet<Key>,
}

impl CameraController {
    fn new(speed: f32) -> Self {
        Self {
            speed,
            pressed_keys: HashSet::new(),
        }
    }

    fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
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
                    self.pressed_keys.remove(&logical_key);
                }
                false
            }
            _ => false,
        }
    }

    fn update_camera(&self, camera: &mut Camera) {
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let mut offset = Vector3::zero();
        let move_delta = 0.1;

        for key in self.pressed_keys.iter() {
            match key {
                Key::Character(char) => match camera.camera_type {
                    CameraType::Orthographic() => match char.to_ascii_lowercase().as_str() {
                        "w" => camera.eye.y += move_delta,
                        "s" => camera.eye.y -= move_delta,
                        "d" => camera.eye.x += move_delta,
                        "a" => camera.eye.x -= move_delta,
                        _ => {}
                    },
                    CameraType::Perspective(_) => match char.to_ascii_lowercase().as_str() {
                        "s" => camera.eye.z -= move_delta,
                        "w" => camera.eye.z += move_delta,
                        "e" => camera.eye.y -= move_delta,
                        "q" => camera.eye.y += move_delta,
                        "a" => camera.eye.x -= move_delta,
                        "d" => camera.eye.x += move_delta,
                        _ => {}
                    },
                },
                _ => {}
            }
        }

        let _ = camera.eye.add(offset);
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
                        1.0 / self.ratio, 0.0, 0.0, 0.0, //x
                        0.0, 1.0, 0.0, 0.0, //y
                        (self.eye.x - self.target.x), (self.eye.y - self.target.y), 1.0, 0.0, //z
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
    render_pipeline: wgpu::RenderPipeline,
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
}

impl<'a> State<'a> {
    // Creating some of the wgpu types requires async code
    async fn new(window: &'a Window) -> State<'a> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
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
                                wgpu::Limits::downlevel_webgl2_defaults()
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

                let camera = Camera {
                    // position the camera 1 unit up and 2 units back
                    // +z is out of the screen
                    eye: (0.0, 0.0, 0.0).into(),
                    // have it look at the origin
                    target: (0.0, 0.0, 0.0).into(),
                    // which way is "up"
                    up: cgmath::Vector3::unit_y(),
                    znear: 0.1,
                    zfar: 100.0,
                    ratio: config.width as f32 / config.height as f32,
                    //camera_type: CameraType::Perspective(45.0),
                    camera_type: CameraType::Orthographic(),
                };

                let camera_controller = CameraController::new(0.2);

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

                let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("TutorialShader"),
                    source: shaders::get(shaders::ShaderType::TutorialShader),
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

                let render_pipeline =
                    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("Render Pipeline"),
                        layout: Some(&render_pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &shader,
                            entry_point: Option::from("vs_main"),
                            buffers: &[model::ModelVertex::desc(), InstanceRaw::desc()],
                            compilation_options: wgpu::PipelineCompilationOptions::default(),
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &shader,
                            entry_point: Option::from("fs_main"),
                            targets: &[Some(wgpu::ColorTargetState {
                                format: config.format,
                                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                                write_mask: wgpu::ColorWrites::ALL,
                            })],
                            compilation_options: wgpu::PipelineCompilationOptions::default(),
                        }),
                        primitive: wgpu::PrimitiveState {
                            topology: wgpu::PrimitiveTopology::TriangleList,
                            strip_index_format: None,
                            front_face: wgpu::FrontFace::Ccw,
                            //cull_mode: Some(wgpu::Face::Back),
                            cull_mode: None,
                            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                            polygon_mode: wgpu::PolygonMode::Fill,
                            // Requires Features::DEPTH_CLIP_CONTROL
                            unclipped_depth: false,
                            // Requires Features::CONSERVATIVE_RASTERIZATION
                            conservative: false,
                        },
                        depth_stencil: Some(wgpu::DepthStencilState {
                            format: texture::Texture::DEPTH_FORMAT,
                            depth_write_enabled: true,
                            depth_compare: wgpu::CompareFunction::Less,
                            stencil: wgpu::StencilState::default(),
                            bias: wgpu::DepthBiasState::default(),
                        }),
                        multisample: wgpu::MultisampleState {
                            count: 1,
                            mask: !0,
                            alpha_to_coverage_enabled: false,
                        },
                        multiview: None,
                        cache: None,
                    });

                let screensaver_type = screensaver::ScreenSaverType::Snow;

                let mut screensaver = match screensaver_type {
                    screensaver::ScreenSaverType::Snow => Box::new(screensaver::SnowScreenSaver {
                        models: Vec::new(),
                    }),
                };

                screensaver.setup(&device, &queue, &texture_bind_group_layout);

                Self {
                    window,
                    surface,
                    device,
                    queue,
                    config,
                    size,
                    background_color,
                    render_pipeline,
                    depth_texture,
                    camera,
                    camera_controller,
                    camera_uniform,
                    camera_buffer,
                    camera_bind_group,
                    screensaver,
                    screensaver_type,
                    last_updated: Instant::now(),
                }
            }
            None => {
                panic!("Unable to find an appropriate graphics adapter");
            }
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
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
        self.camera.ratio = new_size.width as f32 / new_size.height as f32;
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        if !self.camera_controller.process_events(event) {
            match self.screensaver_type {
                ScreenSaverType::Snow => match event {
                    WindowEvent::CursorMoved { position, .. } => {
                        self.camera.target.x = -(position.x as f32 / self.size.width as f32) + 0.5;
                        self.camera.target.y = (position.y as f32 / self.size.height as f32) - 0.5;
                        false
                    }
                    _ => false,
                },
            }
        } else {
            true
        }
    }

    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
        self.screensaver.update(
            &self.device,
            Instant::now().duration_since(self.last_updated),
        );
        self.background_color = self.screensaver.get_background_color();

        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                let size_x = web_sys::window().unwrap().inner_width().unwrap().as_f64().unwrap();
                let size_y = web_sys::window().unwrap().inner_height().unwrap().as_f64().unwrap();
                let scale = web_sys::window().unwrap().device_pixel_ratio();
                
                let  _ = self.window.request_inner_size(winit::dpi::LogicalSize::new(size_x / scale, size_y / scale));
                let _ = self.window.canvas().unwrap().style().set_property(
                    "transform",
                    &*format! {"scale({})", scale},
                );
            }
        }
        self.last_updated = Instant::now();
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

            // lib.rmesh.in
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);

            for model in self.screensaver.get_models() {
                render_pass.set_bind_group(0, &model.material.bind_group, &[]);
                use model::DrawModel;
                render_pass.draw_mesh_instanced(&model.mesh, 0..model.mesh.instances.len() as u32);
            }
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }
    let event_loop = EventLoop::new().unwrap();

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            let canvas = web_sys::window().unwrap().document().unwrap().get_element_by_id("screensaver").unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .map_err(|_| ())
                .unwrap();
            let window = WindowBuilder::new()
                //.with_fullscreen(Some(Fullscreen::Borderless(None)))
                .with_canvas(Some(canvas))
                .build(&event_loop).unwrap();
            }
        else {
            let window = WindowBuilder::new()
                .with_fullscreen(Some(Fullscreen::Borderless(None)))
                //.with_visible(false)
                .build(&event_loop).unwrap();
            window.set_cursor_visible(false);
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        let _ = window.request_inner_size(PhysicalSize::new(450, 400));
    }
    let mut state = State::new(&window).await;

    let result = event_loop
        .run(move |event, control_flow| {
            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == state.window().id() => {
                    if !state.input(event) {
                        match event {
                            WindowEvent::CloseRequested
                            | WindowEvent::MouseInput {
                                state: ElementState::Pressed,
                                ..
                            }
                            | WindowEvent::KeyboardInput {
                                event:
                                    KeyEvent {
                                        state: ElementState::Pressed,
                                        ..
                                    },
                                ..
                            } => {
                                //exit the screensaver when any key is pressed, but not on the web (duh)
                                #[cfg(not(target_arch="wasm32"))]
                                control_flow.exit()
                            },
                            WindowEvent::Resized(physical_size) => {
                                state.resize(*physical_size);
                            }
                            WindowEvent::RedrawRequested => {
                                // This tells winit that we want another frame after this one
                                state.window().request_redraw();

                                state.window().set_visible(true);

                                /*
                                if !surface_configured {
                                    return;
                                }*/

                                state.update();
                                match state.render() {
                                    Ok(_) => {}
                                    // Reconfigure the surface if it's lost or outdated
                                    Err(
                                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                                    ) => state.resize(state.size),
                                    // The system is out of memory, we should probably quit
                                    Err(wgpu::SurfaceError::OutOfMemory) => {
                                        log::error!("OutOfMemory");
                                        control_flow.exit();
                                    }

                                    // This happens when the a frame takes too long to present
                                    Err(wgpu::SurfaceError::Timeout) => {
                                        log::warn!("Surface timeout")
                                    },
                                    Err(wgpu::SurfaceError::Other) => {
                                        log::error!("Other render error ¯\\_(ツ)_/¯")
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
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
