use anymap::AnyMap;
use ship::{Ship, ShipGpuResources};
mod background;
mod bloom;
mod camera;
mod ecs;
mod fullscreen;
mod galactic_transform;
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
mod shader_manager;
mod ship;
mod texture;
mod time;
mod transform;
mod tri_meash_gen;
use crate::camera::Camera;
use background::Background;
use bevy_ecs::{
    component::Component,
    query::{QueryBuilder, With},
    system::{Local, Res},
    world::{Mut, World},
};
use bloom::{Bloom, BloomReasoruces};
use camera::{CameraUniform, CameraUniformInner};

use ecs::Ecs;
use fullscreen::FullscreenQuad;
use gilrs::Gilrs;
use gizmo::{ArrowGizmo, RawArrowInstance};
use glam::{vec3, Mat3, Mat4, Quat, Vec2, Vec3};
use gpu::{Gpu, InstanceBuffer};
use input::{AxisInput, ButtonInput, MouseDelta};
use model::DrawModel;
use model::Model;

use planet_shadows::PlanetShadows;

use post::Postprocessing;
use shader_manager::{PipelineHandle, ShaderManager, ShaderStorage};
use std::{
    mem::size_of,
    sync::{Arc, Mutex},
};
use texture::Texture;
use time::Time;
use transform::Transform;
use wgpu::{
    BindGroup, BindGroupLayout, BufferUsages, Color, Device, PipelineLayout, ShaderModule,
    TextureFormat, VertexBufferLayout,
};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Fullscreen, WindowBuilder},
};

struct StandardRenderPipeline {
    handle: PipelineHandle,
}

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
                state.ecs.world.send_event(MouseDelta {
                    x: delta.0 as f32,
                    y: delta.1 as f32,
                });
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

struct SunRenderReasources {
    sun_transform_instance_buffer: InstanceBuffer,
    sun_render_pipeline: PipelineHandle,
    sun_model: Model,
}

struct ViewTextureReasources {
    view_texture: Texture,
    view_textures_bind_group: BindGroup,
    view_textures_bind_group_layout: wgpu::BindGroupLayout,
    depth_texture: Texture,
}

struct State {
    surface: wgpu::Surface,
    gpu: Gpu,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    window: Window,

    fire_bg: bool,

    render_stuff: AnyMap,

    render_world: World,

    tri_meshes: Vec<tri_meash_gen::TriMesh>,

    // planets: Planets,
    planet_model: Model,
    planet_transform_instance_buffer: Arc<Mutex<InstanceBuffer>>,

    // ship_transforms: Vec<Transform>,
    ship_model: Model,

    // arrows: ArrowGizmos,
    arrow_transform_instance_buffer: InstanceBuffer,
    arrow_model: Model,
    gizmo_render_pipeline: PipelineHandle,

    ecs: Ecs,

    shader_manager: ShaderManager,
    gilrs: Gilrs,
}

impl State {
    // Creating some of the wgpu types requires async code
    async fn new(window: Window) -> Self {
        let mut render_stuff = AnyMap::new();
        let render_world = World::new();
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

        let mut limits = wgpu::Limits::downlevel_defaults();
        limits.max_bind_groups = 5;
        limits.max_texture_dimension_2d = 8192;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::all_webgpu_mask(),
                    limits,
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_capabilities = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_capabilities.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let camera = Camera {
            aspect: config.width as f32 / config.height as f32,
            eye: vec3(0.0, 0.0, -200.0),
            target: Vec3::ZERO,
            ..Default::default()
        };
        let mut ecs = Ecs::new().await;

        let cam = CameraUniform::setup(&camera, &device);
        render_stuff.insert(cam.clone());
        ecs.world.insert_resource(cam);
        ecs.extract.add_systems(camera::update_camera);

        // let camera_controller = PlayerController::new(100.0)

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

        render_stuff.insert({
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

            let color_texture = make_view_texture(&config, &device);
            let depth_texture =
                texture::Texture::create_depth_texture(&device, &config, "depth_texture");
            ViewTextureReasources {
                view_textures_bind_group: make_view_textures_bind_group(
                    &device,
                    &view_textures_bind_group_layout,
                    &depth_texture,
                    &color_texture,
                ),
                view_textures_bind_group_layout,
                depth_texture,
                view_texture: color_texture,
            }
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
            resources::load_model("retroShip.obj", &device, &queue, &texture_bind_group_layout)
                .await
                .unwrap();

        let sun_transforms = vec![Transform {
            scale: Vec3::ONE * 20.0,
            ..Default::default()
        }];
        let planet_transform_instance_buffer = Arc::new(Mutex::new(InstanceBuffer::new(
            Some("Planet instance buffer".to_owned()),
            BufferUsages::VERTEX | BufferUsages::COPY_DST,
            &device,
        )));
        // let ship_transform_instance_buffer = ;

        let sun_transform_buffer_data =
            instance_buffer_bytes_from_components(sun_transforms.iter().map(
                |Transform {
                     scale,
                     translation,
                     rotation,
                 }| (*translation, *rotation, *scale),
            ));
        let sun_transform_instance_buffer = InstanceBuffer::new_init::<RawInstance>(
            Some("Sun instance Buffer".to_owned()),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            &device,
            bytemuck::cast_slice(&sun_transform_buffer_data),
        );

        let arrow_model =
            resources::load_model("arrow.obj", &device, &queue, &texture_bind_group_layout)
                .await
                .unwrap();
        // let arrow_transform_buffer_data = arrows.to_raw_buffer();
        let arrow_transform_instance_buffer = InstanceBuffer::new(
            Some("arrow instance Buffer".to_owned()),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            &device,
        );

        let gpu = Gpu {
            device: Arc::new(device),
            queue: Arc::new(queue),
        };
        render_stuff.insert(gpu.clone());
        ecs.world.insert_resource(gpu.clone());
        ecs.world.insert_resource(Camera::default());

        time::init(&mut ecs).await;
        physics::init(&mut ecs).await;
        planets::init(
            &mut ecs,
            &gpu,
            &texture_bind_group_layout,
            planet_transform_instance_buffer.clone(),
        )
        .await;
        gravity::init(&mut ecs).await;
        input::init(&mut ecs);
        player_controller::init(&mut ecs);
        gizmo::init(&mut ecs);
        ship::init(&mut ecs, &mut render_stuff);

        ecs.update.add_systems(log_time);

        ecs.init.run(&mut ecs.world);

        let mut shader_manager = ShaderManager::new(gpu.device.clone()).unwrap();
        render_stuff.insert(FullscreenQuad::init(&gpu.device, &mut shader_manager));

        if let Some((vt,)) = get_all!(render_stuff, ViewTextureReasources) {
            render_stuff.insert(PlanetShadows::new(
                &gpu.device,
                &config,
                &mut shader_manager,
                &vt.view_textures_bind_group_layout,
            ));
        }

        Background::init(
            &gpu.device,
            &mut shader_manager,
            &mut render_stuff,
            &mut ecs,
        );

        if let Some((vt, ps, bg, fsq)) = get_all!(
            render_stuff,
            ViewTextureReasources,
            PlanetShadows,
            Background,
            FullscreenQuad
        ) {
            render_stuff.insert(Postprocessing::new(
                &gpu.device,
                &config,
                &fsq,
                &vt.view_textures_bind_group_layout,
                &ps.texture_bind_group_layout,
                &bg.texture_cube_bind_group_layout,
                &mut shader_manager,
            ));
        }
        BloomReasoruces::init(&gpu, &config, &mut render_stuff, &mut shader_manager);

        Bloom::init(&config, &gpu.device, &mut render_stuff);

        init_normal_pipeline(
            &mut render_stuff,
            &mut shader_manager,
            &texture_bind_group_layout,
        );

        let gizmo_shader = shader_manager.load_shader("./src/gizmo.wgsl");
        // let tbgl = texture_bind_group_layout.clone();
        // let cgs = camera_gpu_stuff.clone();
        let gizmo_layout = Box::leak(Box::new(mk_pipeline_layout(
            &gpu.device.clone(),
            "gizmo",
            &[&render_stuff
                .get::<CameraUniform>()
                .unwrap()
                .inner
                .bind_group_layout],
        )));
        let gizmo_render_pipeline = shader_manager.add_render_pipeline(
            vec![gizmo_shader],
            Box::new(move |_rr, device, shader_manager| {
                let (gs,) = get_all_shaders!(shader_manager, gizmo_shader).unwrap();
                mk_pipeline(
                    &device,
                    gizmo_layout,
                    None,
                    &gs,
                    "gizmo",
                    RawArrowInstance::desc(),
                )
            }),
        );

        render_stuff.insert({
            let sun_shader = shader_manager.load_shader("./src/sun.wgsl");

            SunRenderReasources {
                sun_transform_instance_buffer,
                sun_render_pipeline: shader_manager.add_render_pipeline(
                    vec![sun_shader],
                    Box::new(move |rr: &AnyMap, device: &Device, sm: &ShaderStorage| {
                        let (cgs,) = get_all!(rr, CameraUniform).unwrap();

                        mk_pipeline(
                            device,
                            &Box::leak(Box::new(mk_pipeline_layout(
                                device,
                                "Sun",
                                &[&cgs.inner.bind_group_layout, &texture_bind_group_layout],
                            ))),
                            None,
                            &sm.get_module(sun_shader).unwrap(),
                            "sun",
                            RawInstance::desc(),
                        )
                    }),
                ),
                sun_model,
            }
        });

        tri_meash_gen::init(&mut shader_manager, &mut render_stuff, &gpu.device);

        let tri_meshes = vec![tri_meash_gen::prepare_mesh(
            &tri_meash_gen::gen_mesh(100, 100, |p| {
                let p = p / 100.0 - 0.5;
                p.length() * 2.0
            }),
            &gpu,
        )];

        Self {
            window,
            surface,
            config,
            size,
            planet_model,

            // planets,
            planet_transform_instance_buffer: planet_transform_instance_buffer.into(),

            render_world,
            // ship_transforms,
            ship_model,
            fire_bg: true,
            // arrows,
            arrow_transform_instance_buffer,
            arrow_model,
            gizmo_render_pipeline,
            ecs,
            gpu,
            gilrs: Gilrs::new().unwrap(),
            shader_manager,
            render_stuff,
            tri_meshes: tri_meshes,
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
                self.render_stuff
                    .get_mut::<CameraUniform>()
                    .unwrap()
                    .update(&camera, &self.gpu.queue);
            }
            self.config.height = new_size.height;
            self.surface.configure(&self.gpu.device, &self.config);

            if let Some(ViewTextureReasources {
                ref mut view_texture,
                ref mut view_textures_bind_group,
                ref mut view_textures_bind_group_layout,
                ref mut depth_texture,
            }) = self.render_stuff.get_mut::<ViewTextureReasources>()
            {
                *depth_texture = texture::Texture::create_depth_texture(
                    &self.gpu.device,
                    &self.config,
                    "depth_texture",
                );
                *view_texture = make_view_texture(&self.config, &self.gpu.device);
                *view_textures_bind_group = make_view_textures_bind_group(
                    &self.gpu.device,
                    &view_textures_bind_group_layout,
                    &depth_texture,
                    &view_texture,
                );
            }
            if let Some(mut bloom) = self.render_stuff.remove::<Bloom>() {
                if let Some(ViewTextureReasources {
                    ref view_texture, ..
                }) = self.render_stuff.get::<ViewTextureReasources>()
                {
                    bloom.resize(
                        &self.config,
                        &self.gpu.device,
                        &view_texture.view,
                        &self.render_stuff,
                    );
                }
                self.render_stuff.insert(bloom);
            }

            if let Some(planet_shadows) = self.render_stuff.get_mut::<PlanetShadows>() {
                planet_shadows.resize(&self.config, &self.gpu.device);
            }
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
                    if *key == VirtualKeyCode::B {
                        self.fire_bg = true;
                    }

                    false
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

        self.shader_manager.construct_piplines(&self.render_stuff);

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

            self.arrow_transform_instance_buffer.write(
                &self.gpu.device,
                &arrow_instances.as_slice(),
                &self.gpu.queue,
            );
        }
        // {
        //     let planet_instances = instance_buffer_bytes(
        //         QueryBuilder::<&Transform, With<Planet>>::new(&mut self.ecs.world)
        //             .build()
        //             .iter(&self.ecs.world),
        //     );
        //     self.planet_transform_instance_buffer.write(
        //         &self.gpu.device,
        //         &planet_instances.as_slice(),
        //         &self.gpu.queue,
        //     );
        // }

        if let Some(planet_shadows) = self.render_stuff.get_mut::<PlanetShadows>() {
            planet_shadows.update(&mut self.ecs.world, &self.gpu.device, &self.gpu.queue);
        }

        self.ecs.world.resource_mut::<ButtonInput>().tick();
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        if self.fire_bg {
            self.fire_bg = false;
            self.render_stuff
                .get_mut::<Background>()
                .unwrap()
                .compute(&mut encoder, &self.shader_manager);
        }

        let output = self.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        // panic!("{}", view.suboptimal);

        let Ok(instance_buffer) = self.planet_transform_instance_buffer.lock() else {
            panic!();
        };
        if let (
            Some(pipeline),
            Some((
                view_rr,
                CameraUniform {
                    inner: camera_uniform,
                },
                Background {
                    texture_cube_bind_group,
                    ..
                },
                sgr,
            )),
        ) = (
            self.render_stuff.get::<StandardRenderPipeline>().and_then(
                |StandardRenderPipeline { handle }| self.shader_manager.get_render_pipeline(handle),
            ),
            get_all!(
                self.render_stuff,
                ViewTextureReasources,
                CameraUniform,
                Background,
                ShipGpuResources
            ),
        ) {
            let bind_group = &camera_uniform.bind_group;

            let ship_instance_buffer = sgr.instance_buffer.lock().unwrap();
            // Planets
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view_rr.view_texture.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &view_rr.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_bind_group(2, &texture_cube_bind_group, &[]);
            render_pass.set_pipeline(&pipeline);

            render_pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

            render_pass.draw_model_instanced(
                &self.planet_model,
                0..instance_buffer.instances,
                &bind_group,
            );

            // Ship as well
            render_pass.set_vertex_buffer(1, ship_instance_buffer.buffer.slice(..));
            render_pass.draw_model_instanced(
                &self.ship_model,
                0..ship_instance_buffer.instances,
                &bind_group,
            );
            drop(render_pass);
        }

        tri_meash_gen::draw_meshes(
            &mut encoder,
            &self.shader_manager,
            &self.tri_meshes,
            &self.render_stuff,
        );

        if let Some((
            sun_rr,
            ViewTextureReasources {
                view_texture,
                depth_texture,
                ..
            },
            CameraUniform {
                inner: bind_group, ..
            },
        )) = get_all!(
            self.render_stuff,
            SunRenderReasources,
            ViewTextureReasources,
            CameraUniform
        ) {
            if let Some(pipeline) = self
                .shader_manager
                .get_render_pipeline(&sun_rr.sun_render_pipeline)
            {
                // Sun
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Sun render pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view_texture.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_texture.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                render_pass
                    .set_vertex_buffer(1, sun_rr.sun_transform_instance_buffer.buffer.slice(..));
                render_pass.set_pipeline(pipeline);

                render_pass.draw_model_instanced(
                    &sun_rr.sun_model,
                    0..sun_rr.sun_transform_instance_buffer.instances,
                    &bind_group.bind_group,
                );
            }
        }

        if let (
            Some(pipeline),
            Some((
                ViewTextureReasources {
                    view_texture,
                    view_textures_bind_group,
                    view_textures_bind_group_layout,
                    depth_texture,
                },
                CameraUniform {
                    inner: bind_group, ..
                },
            )),
        ) = (
            self.shader_manager
                .get_render_pipeline(&self.gizmo_render_pipeline),
            get_all!(self.render_stuff, ViewTextureReasources, CameraUniform),
        ) {
            // Arrow gizmos
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Gizmo render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view_texture.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_texture.view,
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
            render_pass.set_pipeline(pipeline);

            render_pass.draw_model_instanced(
                &self.arrow_model,
                0..self.arrow_transform_instance_buffer.instances,
                &bind_group.bind_group,
            );
        }

        // planet_shadows::draw_planet_shadows(&mut encoder, &self.shader_manager, &self.render_stuff);
        bloom::draw_bloom(&mut encoder, &self.shader_manager, &self.render_stuff);

        post::post_draw_post(
            &mut encoder,
            output_view,
            &self.shader_manager,
            &self.render_stuff,
        );

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn init_normal_pipeline(
    render_stuff: &mut anymap::Map,
    shader_manager: &mut ShaderManager,
    texture_bind_group_layout: &BindGroupLayout,
) {
    struct NormalPiplineLayout {
        layout: PipelineLayout,
    }
    let normal_shader = shader_manager.load_shader("./src/shader.wgsl");

    let (gpu, background) = get_all!(render_stuff, Gpu, Background).unwrap();

    render_stuff.insert(NormalPiplineLayout {
        layout: mk_pipeline_layout(
            &gpu.device.clone(),
            "normal",
            &[
                &render_stuff
                    .get::<CameraUniform>()
                    .unwrap()
                    .inner
                    .bind_group_layout,
                &texture_bind_group_layout.clone(),
                &background.texture_cube_bind_group_layout,
            ],
        ),
    });
    render_stuff.insert(StandardRenderPipeline {
        handle: shader_manager.add_render_pipeline(
            vec![normal_shader],
            Box::new(move |rr, device, shader_manager| {
                let (ns,) = get_all_shaders!(shader_manager, normal_shader).unwrap();
                let (NormalPiplineLayout { layout },) = get_all!(rr, NormalPiplineLayout).unwrap();
                mk_pipeline(&device, layout, None, &ns, "normal", RawInstance::desc())
            }),
        ),
    });
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
    layout: &PipelineLayout,
    format: Option<TextureFormat>,
    shader: &ShaderModule,
    label: &str,
    descritptor: VertexBufferLayout,
) -> wgpu::RenderPipeline {
    let render_pipeline = {
        use model::Vertex;
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(format!("{label} Render Pipeline").as_str()),
            layout: Some(&layout),
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

fn mk_pipeline_layout(
    device: &wgpu::Device,
    label: &str,
    bind_group_layouts: &[&BindGroupLayout],
) -> wgpu::PipelineLayout {
    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(format!("{label} pipline layout").as_str()),
        bind_group_layouts,
        push_constant_ranges: &[],
    });
    render_pipeline_layout
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

    pub fn from_transform(t: &Transform) -> Self {
        Self {
            transform: t.to_mat(),
            rotation_only: Mat3::from_quat(t.rotation),
        }
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    // fn test(x: Vec2) -> f32 {
    //     x.x.sin() * x.y.cos()
    // }
    // let mesh = tri_meash_gen::gen_mesh(5, 5, test);
    // println!("{:#?}", mesh);

    color_eyre::install().unwrap();
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

fn log_time(
    mut current_second: Local<f64>,
    mut frame: Local<u128>,
    mut avg: Local<f64>,
    time: Res<Time>,
) {
    use color_eyre::owo_colors::OwoColorize;
    *frame += 1;
    *avg += time.delta_seconds as f64;
    if time.since_startup > *current_second + 1.0 {
        println!(
            "[Time]: FPS: {}; Delta avg: {}",
            match *frame {
                0..=30 => format!("{}", (*frame).red()),
                31..=60 => format!("{}", (*frame).yellow()),
                61.. => format!("{}", (*frame).green()),
            },
            (*avg / *frame as f64 * 1000.0) as u64
        );
        *frame = 0;
        *avg = 0.0;
        *current_second = time.since_startup;
    }

    if time.delta_seconds > 1.0 / 60.0 {
        println!(
            "[Time] {}: {} millis",
            "Lag spike".red(),
            time.delta_seconds * 1000.0
        );
    }
}

#[macro_export]
macro_rules! get_all {
    ( $map:expr, $( $x:ty ),* ) => {
        {
            let clj = ||{Some(
                (
                    $(

                        {
                            let tmp = $map.get::<$x>();
                            tmp?
                        },
                    )*
                )
            )
            };
            clj()
        }
    }
}

#[macro_export]
macro_rules! idx_all {
    ( $map:expr, $( $x:ty ),* ) => {
        {
                (
                    $(

                        {
                            let tmp = $map.get::<$x>();
                            tmp.expect(&format!("Failed to index type {:?}", std::any::type_name::<$x>()))
                        },
                    )*
                )

        }
    }
}

#[macro_export]
macro_rules! get_all_shaders {
    ( $sm:expr, $( $handle:expr),* ) => {
        {
            let closure = ||{Some(
                (
                    $(
                        $sm.get_module($handle)?,
                    )*
                )
            )
            };
            closure()
        }
    }
}
