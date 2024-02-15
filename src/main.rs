// use crate::model::Vertex;
mod camera;
mod gizmo;
mod model;
mod planets;
mod player_controller;
mod post;
mod resources;
mod texture;
mod transform;
use crate::camera::Camera;
use crate::player_controller::PlayerController;
use bytemuck::cast_slice;
use camera::CameraUniform;
use core::panic;
use gizmo::{ArrowGizmos, RawArrowInstance};
use glam::{vec3, vec4, Mat3, Mat4, Quat, Vec3};
use model::Model;
use planets::{Planet, Planets};
use player_controller::{gravity, G};
use post::Postprocessing;
use rand::prelude::*;
use std::{mem::size_of, sync::Arc};
use texture::Texture;
use transform::Transform;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupLayout, BufferUsages, Color, ShaderModule, TextureFormat,
    VertexBufferLayout,
};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Fullscreen, WindowBuilder},
};

use std::time::Instant;
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
                state.player_controller.mouse_motion(delta);
            }
            _ => {}
            // DeviceEvent::MouseWheel { delta } => todo!(),
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
                        if *has_focus {
                            state
                                .window
                                .set_cursor_grab(winit::window::CursorGrabMode::Locked);
                            state.window.set_cursor_visible(false);
                        } else {
                            state
                                .window
                                .set_cursor_grab(winit::window::CursorGrabMode::None);
                            state.window.set_cursor_visible(true);
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
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    window: Window,
    render_pipeline: wgpu::RenderPipeline,
    sun_render_pipeline: wgpu::RenderPipeline,
    gizmo_render_pipeline: wgpu::RenderPipeline,

    diffuse_bind_group: wgpu::BindGroup,
    diffuse_texture: texture::Texture,
    camera: Camera,
    player_controller: PlayerController,
    camera_uniform: CameraUniform,

    planets: Planets,
    planet_model: Model,
    planet_instance_buffer: wgpu::Buffer,

    ship_transforms: Vec<Transform>,
    ship_instance_buffer: wgpu::Buffer,
    ship_model: Model,

    sun_transforms: Vec<Transform>,
    sun_instance_buffer: wgpu::Buffer,
    sun_model: Model,

    arrows: ArrowGizmos,
    arrow_instance_buffer: wgpu::Buffer,
    arrow_model: Model,

    init_time: Instant,
    timer: Instant,
    delta_time: f32,

    view_texture: Texture,
    depth_texture: Texture,
    view_textures_bind_group: BindGroup,

    post: Postprocessing,
    paused: bool,
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

        let view_texture = make_view_texture(&config, &device);
        let post = Postprocessing::new(&device, &config);
        let view_textures_bind_group = make_view_textures_bind_group(
            &device,
            &post.textures_bind_group_layout,
            &depth_texture,
            &view_texture,
        );

        let camera = Camera {
            aspect: config.width as f32 / config.height as f32,
            eye: vec3(0.0, 0.0, -200.0),
            target: Vec3::ZERO,
            ..Default::default()
        };
        let camera_gpu_stuff = CameraUniform::setup(&camera, &device);

        let camera_controller = PlayerController::new(100.0);

        let diffuse_bytes = include_bytes!("../happy-tree.png");
        let diffuse_texture =
            texture::Texture::from_bytes(&device, &queue, &diffuse_bytes[..], "Happy tree")
                .unwrap();

        let texture_bind_group_layout =
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
                ],
                label: Some("texture_bind_group_layout"),
            });

        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        let planet_model =
            resources::load_model("wallmart.obj", &device, &queue, &texture_bind_group_layout)
                .await
                .unwrap();

        let sun_model =
            resources::load_model("wallmart.obj", &device, &queue, &texture_bind_group_layout)
                .await
                .unwrap();

        let ship_model =
            resources::load_model("ship.obj", &device, &queue, &texture_bind_group_layout)
                .await
                .unwrap();

        let normal_shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let render_pipeline = mk_pipeline(
            &device,
            &texture_bind_group_layout,
            &camera_gpu_stuff,
            &config.format,
            normal_shader,
            "normal",
            RawInstance::desc(),
        );
        let sun_transforms = vec![Transform {
            scale: Vec3::ONE * 20.0,
            ..Default::default()
        }];
        let planets = make_planets();

        let planet_transform_buffer_data = instance_buffer_bytes(
            planets
                .translations
                .iter()
                .zip(planets.radii.iter())
                .zip(planets.rotations.iter())
                .map(|((t, s), r)| (*t, *r, *s * Vec3::ONE)),
        );
        let planet_transform_instance_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&planet_transform_buffer_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let mut ship_trans = Transform {
            scale: Vec3::ONE * 0.1,
            translation: vec3(0., 0., -200.),
            rotation: Default::default(),
        };
        // ship_trans.look_at(Vec3::ZERO, ship_trans.up());
        let ship_transforms = vec![ship_trans];

        let ship_transform_buffer_data = instance_buffer_bytes(ship_transforms.iter().map(
            |Transform {
                 scale,
                 translation,
                 rotation,
             }| (*translation, *rotation, *scale),
        ));
        let ship_transform_instance_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Ship instance Buffer"),
                contents: bytemuck::cast_slice(&ship_transform_buffer_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let sun_transform_buffer_data = instance_buffer_bytes(sun_transforms.iter().map(
            |Transform {
                 scale,
                 translation,
                 rotation,
             }| (*translation, *rotation, *scale),
        ));
        let sun_transform_instance_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sun instance Buffer"),
                contents: bytemuck::cast_slice(&sun_transform_buffer_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let arrows = ArrowGizmos {
            transforms: vec![
                Transform {
                    scale: Vec3::ONE * 0.01,
                    ..Default::default()
                };
                3
            ],
            colors: vec![
                vec4(1.0, 0.5, 0.25, 1.0),
                vec4(0.2, 0.5, 1.0, 1.0),
                vec4(0.2, 1., 0.5, 1.0),
            ],
        };

        let arrow_model =
            resources::load_model("arrow.obj", &device, &queue, &texture_bind_group_layout)
                .await
                .unwrap();
        let arrow_transform_buffer_data = arrows.to_raw_buffer();
        let arrow_transform_instance_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("arrow instance Buffer"),
                contents: bytemuck::cast_slice(&arrow_transform_buffer_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let gizmo_shader = device.create_shader_module(wgpu::include_wgsl!("gizmo.wgsl"));
        let gizmo_render_pipeline = mk_pipeline(
            &device,
            &texture_bind_group_layout,
            &camera_gpu_stuff,
            &config.format,
            gizmo_shader,
            "gizmo",
            RawArrowInstance::desc(),
        );

        let sun_shader = device.create_shader_module(wgpu::include_wgsl!("sun.wgsl"));
        let sun_render_pipeline = mk_pipeline(
            &device,
            &texture_bind_group_layout,
            &camera_gpu_stuff,
            &config.format,
            sun_shader,
            "sun",
            RawInstance::desc(),
        );
        let timer = Instant::now();
        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            sun_render_pipeline,
            planet_model,

            diffuse_bind_group,
            diffuse_texture,
            player_controller: camera_controller,
            camera_uniform: camera_gpu_stuff,
            camera,
            timer,
            init_time: timer,
            delta_time: 0.0,
            planets,
            planet_instance_buffer: planet_transform_instance_buffer,

            depth_texture,
            view_texture,
            view_textures_bind_group,

            ship_transforms,
            ship_instance_buffer: ship_transform_instance_buffer,
            ship_model,
            sun_transforms,
            sun_instance_buffer: sun_transform_instance_buffer,
            sun_model,
            arrows,
            arrow_instance_buffer: arrow_transform_instance_buffer,
            arrow_model,
            gizmo_render_pipeline,
            post,
            paused: true,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.camera.aspect = new_size.width as f32 / new_size.height as f32;
            self.camera_uniform.update(&self.camera, &self.queue);
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
            self.view_texture = make_view_texture(&self.config, &self.device);
            self.view_textures_bind_group = make_view_textures_bind_group(
                &self.device,
                &self.post.textures_bind_group_layout,
                &self.depth_texture,
                &self.view_texture,
            )
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved {
                device_id,
                position,
                modifiers,
            } => true,
            WindowEvent::KeyboardInput { input, .. } => match input {
                KeyboardInput {
                    state,
                    virtual_keycode: Some(key),
                    ..
                } => {
                    let pressed = *state == ElementState::Pressed;
                    if *key == VirtualKeyCode::Space && pressed {
                        self.paused = !self.paused;
                        true
                    } else if *key == VirtualKeyCode::Space && pressed {
                        self.paused = !self.paused;
                        true
                    } else if *key == VirtualKeyCode::R && pressed {
                        self.planets = make_planets();
                        true
                    } else if self.player_controller.process_keys(key, pressed) {
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
        let now = Instant::now();
        let time_since_startup = now.duration_since(self.init_time).as_secs_f32();
        self.delta_time = now.duration_since(self.timer).as_secs_f32();
        self.timer = now;

        if !self.paused {
            for (((velocity, translation), mass), radius) in self
                .planets
                .velocities
                .iter_mut()
                .zip(self.planets.translations.iter())
                .zip(self.planets.masses.iter())
                .zip(self.planets.radii.iter())
            {
                for ((translation_b, mass_b), radius_b) in self
                    .planets
                    .translations
                    .iter()
                    .zip(self.planets.masses.iter())
                    .zip(self.planets.radii.iter())
                {
                    *velocity +=
                        gravity(*translation - *translation_b) * self.delta_time * *mass_b / *mass;
                    // let dir = (*translation - *translation_b).normalize_or_zero();
                    // if dir != Vec3::ZERO {
                    //     *velocity +=
                    //         dir * alt_gravity(translation.distance(*translation_b), *radius, *radius_b);
                    // }
                }
                *velocity += gravity(*translation) * SUN_MASS * self.delta_time / *mass;
            }

            for (translation, velocity) in self
                .planets
                .translations
                .iter_mut()
                .zip(self.planets.velocities.iter_mut())
            {
                // *velocity += self.delta_time * gravity(*translation) * SUN_MASS;
                *translation += *velocity * self.delta_time;
            }
            let raw_instances = instance_buffer_bytes(
                self.planets
                    .translations
                    .iter()
                    .zip(self.planets.radii.iter())
                    .zip(self.planets.rotations.iter())
                    .map(|((t, s), r)| (*t, *r, *s * Vec3::ONE)),
            );
            self.queue.write_buffer(
                &self.planet_instance_buffer,
                0,
                bytemuck::cast_slice(&raw_instances),
            );
        }

        {
            self.player_controller.update_rotational_velocity();
            self.player_controller.update(
                &mut self.camera,
                &mut self.ship_transforms[0],
                self.delta_time,
                self.planets
                    .translations
                    .iter()
                    .zip(self.planets.masses.iter()),
            );
            // self.ship_transforms[1] = Transform {
            //     translation: vec3(10., 50., 100.),
            //     scale: vec3(30., 30., 30.),
            //     ..Default::default()
            // };
            let raw = instance_buffer_bytes(
                self.ship_transforms
                    .iter()
                    .map(|t| (t.translation, t.rotation, t.scale)),
            );
            self.queue
                .write_buffer(&self.ship_instance_buffer, 0, bytemuck::cast_slice(&raw));
            self.camera_uniform.update(&self.camera, &self.queue);
        }
        {
            let vel_dir = self.player_controller.velocity.normalize_or_zero();
            self.arrows.transforms[0].translation =
                self.ship_transforms[0].translation + vel_dir * 0.3;
            self.arrows.transforms[0].look_to(vel_dir, self.camera.up);
            let acc_dir = self.player_controller.acceleration.normalize_or_zero();
            self.arrows.transforms[1].translation =
                self.ship_transforms[0].translation + acc_dir * 0.3;
            self.arrows.transforms[2].translation = self.player_controller.camera_target_debug;
            self.arrows.transforms[2].look_at(self.ship_transforms[0].translation, self.camera.up);

            let raw = self.arrows.to_raw_buffer();
            self.queue
                .write_buffer(&self.arrow_instance_buffer, 0, bytemuck::cast_slice(&raw));
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        // panic!("{}", view.suboptimal);

        let mut encoder = self
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
            render_pass.set_vertex_buffer(1, self.planet_instance_buffer.slice(..));
            render_pass.set_pipeline(&self.render_pipeline);

            use model::DrawModel;
            render_pass.draw_model_instanced(
                &self.planet_model,
                0..self.planets.len() as u32,
                &self.camera_uniform.bind_group,
            );

            // Ship as well
            render_pass.set_vertex_buffer(1, self.ship_instance_buffer.slice(..));
            render_pass.draw_model_instanced(
                &self.ship_model,
                0..self.ship_transforms.len() as u32,
                &self.camera_uniform.bind_group,
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
            render_pass.set_vertex_buffer(1, self.sun_instance_buffer.slice(..));
            render_pass.set_pipeline(&self.sun_render_pipeline);

            use model::DrawModel;
            render_pass.draw_model_instanced(
                &self.sun_model,
                0..self.sun_transforms.len() as u32,
                &self.camera_uniform.bind_group,
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
            render_pass.set_vertex_buffer(1, self.arrow_instance_buffer.slice(..));
            render_pass.set_pipeline(&self.gizmo_render_pipeline);

            use model::DrawModel;
            render_pass.draw_model_instanced(
                &self.arrow_model,
                0..self.arrows.len() as u32,
                &self.camera_uniform.bind_group,
            );
        }

        self.post.draw(
            &mut encoder,
            output_view,
            &self.camera_uniform.bind_group,
            &self.view_textures_bind_group,
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn make_planets() -> Planets {
    let mut rng = thread_rng();
    let planets = (0..200)
        .map(|_| {
            let distance = rng.gen_range(30.0..300.0);
            let mut trans = Transform {
                rotation: Quat::from_euler(
                    glam::EulerRot::XYZ,
                    rng.gen_range(-0.3..0.3),
                    rng.gen_range(0.0..std::f32::consts::TAU),
                    rng.gen_range(-0.3..0.3),
                ),
                ..Default::default()
            };
            let f = trans.forward();
            let radius = rng.gen_range(0.3..0.9_f32).powi(7) * 10.0;
            trans.translation = f * distance;
            let mass = radius * radius * radius * 10.0;
            let t = Planet {
                velocity: trans.left() * 0.0005 / trans.translation.length().sqrt() * SUN_MASS * G,
                // * mass.sqrt(),
                mass,
                translation: trans.translation,
                rotation: trans.rotation,
                radius,
            };
            t
        })
        .collect::<Planets>();
    // planets.scales[1] = Vec3::ONE;
    planets
}

fn make_view_textures_bind_group(
    device: &wgpu::Device,
    depth_texture_bind_group_layout: &BindGroupLayout,
    depth_texture: &Texture,
    color_texture: &Texture,
) -> BindGroup {
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        // 4.
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: 100.0,
        ..Default::default()
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &depth_texture_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&color_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&depth_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(&sampler),
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
        format: TextureFormat::Bgra8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT // 3.
        | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    let texture = device.create_texture(&desc);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
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
    texture_bind_group_layout: &wgpu::BindGroupLayout,
    camera_gpu_stuff: &CameraUniform,
    format: &TextureFormat,
    shader: ShaderModule,
    label: &str,
    descritptor: VertexBufferLayout,
) -> wgpu::RenderPipeline {
    let render_pipeline = {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(format!("{label} pipline layout").as_str()),
                bind_group_layouts: &[
                    texture_bind_group_layout,
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
                    format: *format,
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

fn instance_buffer_bytes<'a>(items: impl Iterator<Item = (Vec3, Quat, Vec3)>) -> Vec<RawInstance> {
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

// Add this if code breaks
// unsafe impl bytemuck::Pod for Vertex {}
// unsafe impl bytemuck::Zeroable for Vertex {}

// lib.rs

// struct Model {
//     verts: Arc<wgpu::Buffer>,
//     index: Arc<wgpu::Buffer>,
//     index_len: u32,
// }

// #[repr(C)]
// // This is so we can store this in a buffer
// #[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// struct ModelTransformUniform {
//     matrix: Mat4,
// }

// impl ModelTransformUniform {
//     fn new() -> Self {
//         Self {
//             matrix: Mat4::IDENTITY,
//         }
//     }

//     fn update(&mut self, transform: &Transform) {
//         self.matrix = transform.to_mat();
//     }
// }

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

#[tokio::main]
async fn main() {
    run().await;
}
