// use crate::model::Vertex;
mod bloom;
mod camera;
mod ecs;
mod fullscreen;
mod gizmo;
mod gpu;
mod gravity;
mod input;
mod model;
mod physics;
mod planet_shadows;
mod planets;
mod player_controller;
mod post;
mod resources;
mod shader_loader;
mod texture;
mod time;
mod transform;
use crate::{camera::Camera, player_controller::PlayerController};
use bevy_ecs::{
    component::Component,
    query::{QueryBuilder, With},
    system::{Local, Res},
    world::Mut,
};
use bloom::Bloom;
use camera::CameraUniform;

use ecs::Ecs;
use fullscreen::FullscreenQuad;
use gilrs::Gilrs;
use gizmo::{ArrowGizmo, RawArrowInstance};
use glam::{vec3, Mat3, Mat4, Quat, Vec3};
use gpu::{Gpu, InstanceBuffer};
use input::{AxisInput, ButtonInput};
use model::Model;

use planets::Planet;

use post::Postprocessing;
use shader_loader::{PipelineHandle, ShaderLoader};
use std::{mem::size_of, sync::Arc};
use texture::Texture;
use time::Time;
use transform::Transform;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupLayout, BufferDescriptor, BufferUsages, Color, ShaderModule, TextureFormat,
    VertexBufferLayout,
};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Fullscreen, WindowBuilder},
};

pub async fn run() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    window.set_fullscreen(Some(Fullscreen::Borderless(None)));
    let mut state = State::new(window).await;

    event_loop.run(move |event, _, control_flow| match event {
        Event::RedrawRequested(window_id) if window_id == state.window().id() => {
            state.update();
            match state.render() {
                Ok(_) => {}
                // Reconfigure the surface if lost
                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                // The system is out of memory, we should probably quit
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                // All other errors (Outdated, Timeout) should be resolved by the next frame
                Err(e) => eprintln!("{:?}", e),
            }
        }
        Event::MainEventsCleared => {
            // RedrawRequested will only trigger once unless we manually
            // request it.
            state.window().request_redraw();
        }
        Event::DeviceEvent { event, .. } => match event {
            DeviceEvent::MouseMotion { delta } => {
                // state
                //     .ecs
                //     .world
                //     .resource_mut::<PlayerController>()

                QueryBuilder::<&mut PlayerController>::new(&mut state.ecs.world)
                    .build()
                    .single_mut(&mut state.ecs.world)
                    .mouse_motion(delta);
            }
            _ => {} // DeviceEvent::MouseWheel { delta } => todo!(),
                    // DeviceEvent::Motion { axis, value } => todo!(),
                    // DeviceEvent::Button { button, state } => todo!(),
                    // DeviceEvent::Key(_) => todo!(),
                    // DeviceEvent::Text { codepoint } => todo!(),
        },
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == state.window.id() => {
            if !state.input(event) {
                match event {
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        // new_inner_size is &&mut so we have to dereference it twice
                        state.resize(**new_inner_size);
                    }
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Focused(has_focus) => {
                        if let Err(e) = if *has_focus {
                            state.window.set_cursor_visible(false);
                            state
                                .window
                                .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                        } else {
                            state.window.set_cursor_visible(true);
                            state
                                .window
                                .set_cursor_grab(winit::window::CursorGrabMode::None)
                        } {
                            println!("Error: {e:#?}");
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    });
}

// lib.rs
use winit::window::Window;

const SUN_MASS: f32 = 20000.0;

struct State {
    surface: wgpu::Surface,
    gpu: Gpu,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    window: Window,
    render_pipeline: PipelineHandle,
    sun_render_pipeline: wgpu::RenderPipeline,
    gizmo_render_pipeline: wgpu::RenderPipeline,

    camera_gpu_stuff: Arc<CameraUniform>,

    // planets: Planets,
    planet_model: Model,
    planet_transform_instance_buffer: InstanceBuffer,

    // ship_transforms: Vec<Transform>,
    ship_transform_instance_buffer: InstanceBuffer,
    ship_model: Model,

    sun_transform_instance_buffer: InstanceBuffer,
    sun_model: Model,

    // arrows: ArrowGizmos,
    arrow_transform_instance_buffer: InstanceBuffer,
    arrow_model: Model,

    view_texture: Texture,
    depth_texture: Texture,
    view_textures_bind_group: BindGroup,
    view_textures_bind_group_layout: wgpu::BindGroupLayout,

    fullscreen_quad: FullscreenQuad,

    // planet_shadows: PlanetShadows,
    post: Postprocessing,

    // bloom: Bloom,
    bloom: Bloom,

    initialized: bool,

    bloom_debug: (bool, usize),
    ecs: Ecs,

    shader_loader: ShaderLoader,
    gilrs: Gilrs,
}

impl State {
    // Creating some of the wgpu types requires async code
    async fn new(window: Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window, so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web, we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
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
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");
        let view_textures_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Depth,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("post_texture_bind_group_layout"),
            });
        let view_texture = make_view_texture(&config, &device);
        let view_textures_bind_group = make_view_textures_bind_group(
            &device,
            &view_textures_bind_group_layout,
            &depth_texture,
            &view_texture,
        );

        let camera = Camera {
            aspect: config.width as f32 / config.height as f32,
            eye: vec3(0.0, 0.0, -200.0),
            target: Vec3::ZERO,
            ..Default::default()
        };
        let camera_gpu_stuff = Arc::new(CameraUniform::setup(&camera, &device));

        // let camera_controller = PlayerController::new(100.0);

        let texture_bind_group_layout = Arc::new(device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
            },
        ));

        let planet_model =
            resources::load_model("wallmart.obj", &device, &queue, &texture_bind_group_layout)
                .await
                .unwrap();

        let sun_model =
            resources::load_model("wallmart.obj", &device, &queue, &texture_bind_group_layout)
                .await
                .unwrap();

        let ship_model =
            resources::load_model("nuShip.obj", &device, &queue, &texture_bind_group_layout)
                .await
                .unwrap();

        // let normal_shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let sun_transforms = vec![Transform {
            scale: Vec3::ONE * 20.0,
            ..Default::default()
        }];
        // let planets = make_planets();

        // let planet_transform_buffer_data = instance_buffer_bytes_from_components(
        //     planets
        //         .translations
        //         .iter()
        //         .zip(planets.radii.iter())
        //         .zip(planets.rotations.iter())
        //         .map(|((t, s), r)| (*t, *r, *s * Vec3::ONE)),
        // );
        let planet_transform_instance_buffer = InstanceBuffer {
            buffer: device.create_buffer(&BufferDescriptor {
                label: Some("Planet instance buffer"),
                size: 0,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            instances: 96,
        };
        // let ship_trans =         // ship_trans.look_at(Vec3::ZERO, ship_trans.up());
        // let ship_transforms = vec![ship_trans];

        // let ship_transform_buffer_data =
        //     instance_buffer_bytes_from_components(ship_transforms.iter().map(
        //         |Transform {
        //              scale,
        //              translation,
        //              rotation,
        //          }| (*translation, *rotation, *scale),
        //     ));
        let ship_transform_instance_buffer = InstanceBuffer {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Ship instance Buffer"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
                size: std::mem::size_of::<RawInstance>() as u64,
            }),
            instances: 1,
        };

        let sun_transform_buffer_data =
            instance_buffer_bytes_from_components(sun_transforms.iter().map(
                |Transform {
                     scale,
                     translation,
                     rotation,
                 }| (*translation, *rotation, *scale),
            ));
        let sun_transform_instance_buffer = InstanceBuffer {
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sun instance Buffer"),
                contents: bytemuck::cast_slice(&sun_transform_buffer_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }),
            instances: 1,
        };

        let arrow_model =
            resources::load_model("arrow.obj", &device, &queue, &texture_bind_group_layout)
                .await
                .unwrap();
        // let arrow_transform_buffer_data = arrows.to_raw_buffer();
        let arrow_transform_instance_buffer = InstanceBuffer {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("arrow instance Buffer"),
                mapped_at_creation: false,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                size: std::mem::size_of::<gizmo::RawArrowInstance>() as u64 * 3,
            }),
            instances: 3,
        };

        let gizmo_shader = device.create_shader_module(wgpu::include_wgsl!("gizmo.wgsl"));
        let gizmo_render_pipeline = mk_pipeline(
            &device,
            texture_bind_group_layout.clone(),
            camera_gpu_stuff.clone(),
            None,
            &gizmo_shader,
            "gizmo",
            RawArrowInstance::desc(),
        );

        let sun_shader = device.create_shader_module(wgpu::include_wgsl!("sun.wgsl"));
        let sun_render_pipeline = mk_pipeline(
            &device,
            texture_bind_group_layout.clone(),
            camera_gpu_stuff.clone(),
            None,
            &sun_shader,
            "sun",
            RawInstance::desc(),
        );

        // let planet_shadows =
        //     PlanetShadows::new(&device, &config, &camera_gpu_stuff.bind_group_layout);

        let mut ecs = Ecs::new().await;
        let gpu = Gpu {
            device: Arc::new(device),
            queue: Arc::new(queue),
        };
        ecs.world.insert_resource(gpu.clone());
        ecs.world.insert_resource(Camera::default());

        time::init(&mut ecs).await;
        physics::init(&mut ecs).await;
        planets::init(&mut ecs, &gpu, &texture_bind_group_layout).await;
        gravity::init(&mut ecs).await;
        input::init(&mut ecs);
        player_controller::init(&mut ecs);
        gizmo::init(&mut ecs);

        ecs.update.add_systems(print_delta);

        ecs.init.run(&mut ecs.world);

        let mut shader_loader = ShaderLoader::new(gpu.device.clone()).unwrap();
        let fullscreen_quad = FullscreenQuad::new(&gpu.device, &mut shader_loader);
        let post = Postprocessing::new(
            &gpu.device,
            &config,
            &fullscreen_quad,
            &view_textures_bind_group_layout,
            &mut shader_loader,
        );
        let bloom = Bloom::new(
            &gpu.device,
            &config,
            &fullscreen_quad,
            &view_texture.view,
            &mut shader_loader,
        );
        let normal_shader = shader_loader.load_shader("./src/shader.wgsl".into());
        let cgs = camera_gpu_stuff.clone();
        let render_pipeline = shader_loader.add_pipeline(
            vec![normal_shader],
            Box::new(move |shaders, device| {
                mk_pipeline(
                    &device,
                    texture_bind_group_layout.clone(),
                    cgs.clone(),
                    None,
                    shaders[0],
                    "normal",
                    RawInstance::desc(),
                )
            }),
        );

        Self {
            window,
            surface,
            config,
            size,
            render_pipeline,
            sun_render_pipeline,
            planet_model,

            camera_gpu_stuff,
            // planets,
            planet_transform_instance_buffer,
            depth_texture,
            view_texture,
            view_textures_bind_group,

            // ship_transforms,
            ship_transform_instance_buffer,
            ship_model,
            sun_transform_instance_buffer,
            sun_model,
            // arrows,
            arrow_transform_instance_buffer,
            arrow_model,
            gizmo_render_pipeline,
            post,
            initialized: false,
            fullscreen_quad,
            bloom,
            view_textures_bind_group_layout,
            bloom_debug: (false, 0),
            ecs,
            gpu,
            gilrs: Gilrs::new().unwrap(),
            shader_loader,
            // shader_loader,
            // planet_shadows,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            {
                let mut camera = self.ecs.world.resource_mut::<Camera>();
                camera.aspect = new_size.width as f32 / new_size.height as f32;
                self.camera_gpu_stuff.update(&camera, &self.gpu.queue);
            }
            self.config.height = new_size.height;
            self.surface.configure(&self.gpu.device, &self.config);
            self.depth_texture = texture::Texture::create_depth_texture(
                &self.gpu.device,
                &self.config,
                "depth_texture",
            );
            self.view_texture = make_view_texture(&self.config, &self.gpu.device);
            self.view_textures_bind_group = make_view_textures_bind_group(
                &self.gpu.device,
                &self.view_textures_bind_group_layout,
                &self.depth_texture,
                &self.view_texture,
            );
            self.bloom
                .resize(&self.config, &self.gpu.device, &self.view_texture.view);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            // WindowEvent::CursorMoved {
            //     device_id,
            //     position,
            //     modifiers,
            // } => true,
            WindowEvent::KeyboardInput { input, .. } => match input {
                KeyboardInput {
                    state,
                    virtual_keycode: Some(key),
                    ..
                } => {
                    let mut kb: Mut<ButtonInput> = self.ecs.world.resource_mut();
                    match state {
                        ElementState::Pressed => kb.on_key_down(*key),
                        ElementState::Released => kb.on_key_up(*key),
                    };

                    let pressed = *state == ElementState::Pressed;
                    if *key == VirtualKeyCode::R && pressed {
                        // self.planets = make_planets();
                        true
                    } else if *key == VirtualKeyCode::N && pressed {
                        self.bloom_debug.0 = !self.bloom_debug.0;
                        true
                    } else if *key == VirtualKeyCode::Plus && pressed {
                        self.bloom_debug.1 += 1;
                        true
                    } else if *key == VirtualKeyCode::Minus && pressed {
                        self.bloom_debug.1 = self.bloom_debug.1.saturating_sub(1);
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn update(&mut self) {
        self.ecs
            .world
            .resource_mut::<AxisInput>()
            .update(&mut self.gilrs);
        self.ecs.update();

        self.shader_loader.construct_piplines();

        {
            let ship_bytes = instance_buffer_bytes(
                QueryBuilder::<&Transform, With<Ship>>::new(&mut self.ecs.world)
                    .build()
                    .iter(&self.ecs.world),
            );
            self.gpu.queue.write_buffer(
                &self.ship_transform_instance_buffer.buffer,
                0,
                bytemuck::cast_slice(&ship_bytes),
            );
        }
        {
            // Camera
            let camera = self.ecs.world.resource();
            self.camera_gpu_stuff.update(camera, &self.gpu.queue);
        }
        {
            let arrow_instances =
                QueryBuilder::<(&Transform, &ArrowGizmo)>::new(&mut self.ecs.world)
                    .build()
                    .iter(&self.ecs.world)
                    .map(|(transform, gizmo)| RawArrowInstance {
                        transform: transform.to_mat(),
                        color: gizmo.color,
                    })
                    .collect::<Vec<_>>();
            let arrow_bytes: &[u8] = bytemuck::cast_slice(&arrow_instances);

            self.gpu.queue.write_buffer(
                &self.arrow_transform_instance_buffer.buffer,
                0,
                bytemuck::cast_slice(&arrow_bytes),
            );
        }
        {
            let planet_instances = instance_buffer_bytes(
                QueryBuilder::<&Transform, With<Planet>>::new(&mut self.ecs.world)
                    .build()
                    .iter(&self.ecs.world),
            );
            self.planet_transform_instance_buffer.instances = planet_instances.len() as u32;
            let bytes = bytemuck::cast_slice(&planet_instances);
            if self.planet_transform_instance_buffer.buffer.size() < bytes.len() as u64 {
                println!("New buffer added");
                self.planet_transform_instance_buffer.buffer =
                    self.gpu.device.create_buffer_init(&BufferInitDescriptor {
                        label: Some("Planet instance Buffer"),
                        contents: &bytes,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });
            } else {
                self.gpu.queue.write_buffer(
                    &self.planet_transform_instance_buffer.buffer,
                    0,
                    bytes,
                );
            }
        }
        self.ecs.world.resource_mut::<ButtonInput>().tick();
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if !self.initialized {
            self.update();
            self.initialized = true;
        }
        let output = self.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        // panic!("{}", view.suboptimal);

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            // Planets
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.view_texture.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            if let Some(pipeline) = self.shader_loader.get_pipeline(&self.render_pipeline) {
                render_pass
                    .set_vertex_buffer(1, self.planet_transform_instance_buffer.buffer.slice(..));

                render_pass.set_pipeline(&pipeline);
            }

            use model::DrawModel;
            render_pass.draw_model_instanced(
                &self.planet_model,
                0..self.planet_transform_instance_buffer.instances,
                &self.camera_gpu_stuff.bind_group,
            );

            // Ship as well
            render_pass.set_vertex_buffer(1, self.ship_transform_instance_buffer.buffer.slice(..));
            render_pass.draw_model_instanced(
                &self.ship_model,
                0..self.ship_transform_instance_buffer.instances,
                &self.camera_gpu_stuff.bind_group,
            );

            // 3.
        }
        {
            // Sun
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Sun render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.view_texture.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_vertex_buffer(1, self.sun_transform_instance_buffer.buffer.slice(..));
            render_pass.set_pipeline(&self.sun_render_pipeline);

            use model::DrawModel;
            render_pass.draw_model_instanced(
                &self.sun_model,
                0..self.sun_transform_instance_buffer.instances,
                &self.camera_gpu_stuff.bind_group,
            );

            // render_pass.set_vertex_buffer(1, self.ship_instance_buffer.slice(..));
            // render_pass.draw_model_instanced(
            //     &self.ship_model,
            //     0..self.ship_transforms.len() as u32,
            //     &self.camera_uniform.bind_group,
            // );

            // 3.
        }

        {
            // Arrow gizmos
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Gizmo render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.view_texture.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_vertex_buffer(1, self.arrow_transform_instance_buffer.buffer.slice(..));
            render_pass.set_pipeline(&self.gizmo_render_pipeline);

            use model::DrawModel;
            render_pass.draw_model_instanced(
                &self.arrow_model,
                0..self.arrow_transform_instance_buffer.instances,
                &self.camera_gpu_stuff.bind_group,
            );
        }

        // self.bloom
        //     .draw(&mut encoder, &self.gpu.queue, &self.fullscreen_quad);

        self.post.draw(
            &mut encoder,
            output_view,
            &self.fullscreen_quad,
            &self.camera_gpu_stuff.bind_group,
            &self.view_textures_bind_group,
            if self.bloom_debug.0 {
                &self.bloom.down_texture_bind_groups[self
                    .bloom_debug
                    .1
                    .min(self.bloom.down_texture_bind_groups.len() - 1)]
            } else {
                &self.bloom.up_texture_bind_groups[self
                    .bloom_debug
                    .1
                    .min(self.bloom.up_texture_bind_groups.len() - 1)]
            },
            &self.shader_loader,
        );

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn make_view_textures_bind_group(
    device: &wgpu::Device,
    depth_texture_bind_group_layout: &BindGroupLayout,
    depth_texture: &Texture,
    color_texture: &Texture,
) -> BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &depth_texture_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&color_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&color_texture.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&depth_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(&depth_texture.sampler),
            },
        ],
        label: Some("view_bind_group"),
    })
}

fn make_view_texture(config: &wgpu::SurfaceConfiguration, device: &wgpu::Device) -> Texture {
    let size = wgpu::Extent3d {
        // 2.
        width: config.width,
        height: config.height,
        depth_or_array_layers: 1,
    };
    let desc = wgpu::TextureDescriptor {
        label: Some("View tex"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT // 3.
        | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    let texture = device.create_texture(&desc);
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("View texture descriptor"),
        format: Some(TextureFormat::Rgba16Float),
        ..Default::default()
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        // 4.
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        compare: None, // 5.
        lod_min_clamp: 0.0,
        lod_max_clamp: 100.0,
        ..Default::default()
    });
    Texture {
        texture,
        view,
        sampler,
    }
}

fn mk_pipeline(
    device: &wgpu::Device,
    texture_bind_group_layout: Arc<wgpu::BindGroupLayout>,
    camera_gpu_stuff: Arc<CameraUniform>,
    format: Option<TextureFormat>,
    shader: &ShaderModule,
    label: &str,
    descritptor: VertexBufferLayout,
) -> wgpu::RenderPipeline {
    let render_pipeline = {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(format!("{label} pipline layout").as_str()),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_gpu_stuff.bind_group_layout,
                    // &model_transform_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        use model::Vertex;
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(format!("{label} Render Pipeline").as_str()),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",                              // 1.
                buffers: &[model::ModelVertex::desc(), descritptor], // 2.
            },
            fragment: Some(wgpu::FragmentState {
                // 3.
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    // 4.
                    format: format.unwrap_or(TextureFormat::Rgba16Float),
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                // cull_mode: Some(wgpu::Face::Back),
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
                depth_compare: wgpu::CompareFunction::Greater, // 1.
                stencil: wgpu::StencilState::default(),        // 2.
                bias: wgpu::DepthBiasState::default(),
            }), // 1.
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5.
        })
    };
    render_pipeline
}

fn instance_buffer_bytes_from_components<'a>(
    items: impl Iterator<Item = (Vec3, Quat, Vec3)>,
) -> Vec<RawInstance> {
    items
        .map(|(translation, rotation, scale)| {
            let t = Transform {
                translation,
                rotation,
                scale,
            };
            RawInstance {
                transform: t.to_mat(),
                rotation_only: Mat3::from_quat(t.rotation),
            }
        })
        .collect::<Vec<_>>()
}

fn instance_buffer_bytes<'a>(items: impl Iterator<Item = &'a Transform>) -> Vec<RawInstance> {
    items
        .map(|t| RawInstance {
            transform: t.to_mat(),
            rotation_only: Mat3::from_quat(t.rotation),
        })
        .collect::<Vec<_>>()
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct RawInstance {
    transform: Mat4,
    rotation_only: Mat3,
}
unsafe impl bytemuck::Pod for RawInstance {}
unsafe impl bytemuck::Zeroable for RawInstance {}

impl RawInstance {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 5,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: size_of::<[f32; 16 + 3]>() as wgpu::BufferAddress,
                    shader_location: 10,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: size_of::<[f32; 16 + 6]>() as wgpu::BufferAddress,
                    shader_location: 11,
                },
            ],
        }
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    run().await;
}

// fn main() {
//     let rt = tokio::runtime::Builder::new_current_thread()
//         .enable_all()
//         .on_thread_park(|| println!("Parked"))
//         .build()
//         .unwrap();

//     rt.handle().spawn(async { run().await });
// }

#[derive(Component)]
pub struct Ship;

fn print_delta(
    mut current_second: Local<f64>,
    mut frame: Local<u128>,
    mut avg: Local<f64>,
    time: Res<Time>,
) {
    *frame += 1;
    *avg += time.delta_seconds as f64;
    if time.since_startup > *current_second + 1.0 {
        println!(
            "FPS: {}; Delta avg: {}",
            *frame,
            (*avg / *frame as f64 * 1000.0) as u64
        );
        *frame = 0;
        *avg = 0.0;
        *current_second = time.since_startup;
    }

    if time.delta_seconds > 1.0 / 60.0 {
        println!("Lag spike: {} millis", time.delta_seconds * 1000.0);
    }
}
