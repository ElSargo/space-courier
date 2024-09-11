use anymap::AnyMap;
use gpu::{DeviceRes, QueueRes};
use ship::{Ship, ShipGpuResources};
use sun::SunRenderReasources;
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
mod sun;
mod texture;
mod time;
mod transform;
mod tri_meash_gen;
use crate::{camera::Camera, shader_manager::UnifiedPipelineModuleResource};
use background::Background;
use bevy_ecs::{
    query::QueryBuilder,
    schedule::{Schedule, ScheduleLabel},
    system::{Commands, In, Local, Res, Resource},
    world::{Mut, World},
};
use bloom::Bloom;
use camera::CameraUniform;

use ecs::Ecs;

use gilrs::Gilrs;
use gizmo::{ArrowGizmo, RawArrowInstance};
use glam::{vec3, Mat3, Mat4, Quat, Vec2, Vec3, Vec4};
use input::{AxisInput, ButtonInput, MouseDelta};
use model::DrawModel;

use planet_shadows::PlanetShadows;

use shader_manager::{
    PipelineHandle, ShaderManager, ShaderManagerCtrl, UnifiedPipelineModuleDescriptor,
};
use std::{mem::size_of, sync::Arc};
use texture::Texture;
use time::Time;
use transform::Transform;
use wgpu::{
    BindGroup, BindGroupLayout, Color, Device, PipelineLayout, QueryType, ShaderModule,
    TextureFormat, VertexBufferLayout,
};
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::WindowAttributes,
};

struct StandardRenderPipeline {
    handle: PipelineHandle,
}

pub async fn run() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    // ActiveEventLoop::create_window(ActiveEventLoop::into(), WindowAttributes::default());
    // let window = event_loop.create_window();
    // window.set_fullscreen(Some(Fullscreen::Borderless(None)));
    // let mut state = State::new(window).await;

    // event_loop.run();
}

// lib.rs
use winit::window::Window;

struct ViewTextureReasources {
    view_texture: Texture,
    view_textures_bind_group: BindGroup,
    view_textures_bind_group_layout: wgpu::BindGroupLayout,
    depth_texture: Texture,
}

struct GraphicsContext<'a> {
    surface: wgpu::Surface<'a>,
    window: Window,
    device: DeviceRes,
    queue: QueueRes,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    adapter: wgpu::Adapter,
    shader_manager: ShaderManager,
}

impl GraphicsContext<'_> {
    async fn init<'a>(state: &'a mut State<'a>, event_loop: &ActiveEventLoop) {
        state.graphics_ctx = Some(unsafe { std::mem::MaybeUninit::zeroed().assume_init() });
        let g_ctx = state.graphics_ctx.as_mut().unwrap();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        g_ctx.window = event_loop
            .create_window(WindowAttributes::default())
            .unwrap();

        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window, so this should be safe.
        g_ctx.surface = instance.create_surface(&g_ctx.window).unwrap();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        g_ctx.adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&g_ctx.surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let surface_capabilities = g_ctx.surface.get_capabilities(&g_ctx.adapter);

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
        let size = g_ctx.window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let mut limits = wgpu::Limits::downlevel_defaults();
        limits.max_bind_groups = 5;
        limits.max_texture_dimension_2d = 8192;
        let (device, queue) = g_ctx
            .adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::all_webgpu_mask(),
                    required_limits: limits,
                    label: None,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        g_ctx.surface.configure(&device, &config);

        let device_res: Arc<Device> = device.into();
        g_ctx.device = DeviceRes(device_res.clone());
        g_ctx.queue = QueueRes(queue.into());
        g_ctx.shader_manager = ShaderManager::new(device_res).unwrap();
    }
}

struct State<'a> {
    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    graphics_ctx: Option<GraphicsContext<'a>>,
    ecs: Ecs,
    gilrs: Gilrs,
}

impl ApplicationHandler for State<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.graphics_ctx.is_none() {
            GraphicsContext::init(self, event_loop);
        }

        // surface: Option<wgpu::Surface<'a>>,
        // window: Option<Window>,
        // gpu: Option<Gpu>,
        // config: Option<wgpu::SurfaceConfiguration>,
        // size: Option<winit::dpi::PhysicalSize<u32>>,
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.ecs.world.send_event(MouseDelta {
                    x: delta.0 as f32,
                    y: delta.1 as f32,
                });
            }
            _ => {} // DeviceEvent::MouseWheel { delta } => todo!(),
                    // DeviceEvent::Motion { axis, value } => todo!(),
                    // DeviceEvent::Button { button, self } => todo!(),
                    // DeviceEvent::Key(_) => todo!(),
                    // DeviceEvent::Text { codepoint } => todo!(),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested/*(window_id) if window_id == self.window().id() */=> {
                self.update();
                match self.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => self.resize(self.graphics_ctx.unwrap().size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            },
            WindowEvent::Resized(physical_size) => {
                            // WindowEvent::Resized() => {
                            // RedrawRequested will only trigger once unless we manually
                            // request it.
                            // self.window().request_redraw();
                            // }
                            self.resize(physical_size);
                        },
                        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                            // new_inner_size is &&mut so we have to dereference it twice
                            // self.resize(**new_inner_size);
                        },
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    logical_key: Key::Named(NamedKey::Escape),
                                    state: ElementState::Pressed,
                                    ..
                                },
                            ..
                        } => event_loop.exit(),
                        WindowEvent::Focused(has_focus) => {
                            let window = &self.graphics_ctx.unwrap().window;
                            if let Err(e) = if has_focus {
                                window.set_cursor_visible(false);
                                window
                                    .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                            } else {
                                window.set_cursor_visible(true);
                                window
                                    .set_cursor_grab(winit::window::CursorGrabMode::None)
                            } {
                                println!("Error: {e:#?}");
                            }
                        },
            _ => {}
        }

        todo!()
    }
}

impl<'a> State<'a> {
    // Creating some of the wgpu types requires async code
    async fn new() -> Self {
        let mut ecs = Ecs::new();
        ecs.update.add_systems(log_time);

        Self {
            graphics_ctx: None,
            ecs,
            gilrs: Gilrs::new().unwrap(),
        }

        // // let camera_controller = PlayerController::new(100.0)

        // let texture_bind_group_layout = Arc::new(device.create_bind_group_layout(
        //     &wgpu::BindGroupLayoutDescriptor {
        //         entries: &[
        //             wgpu::BindGroupLayoutEntry {
        //                 binding: 0,
        //                 visibility: wgpu::ShaderStages::FRAGMENT,
        //                 ty: wgpu::BindingType::Texture {
        //                     multisampled: false,
        //                     view_dimension: wgpu::TextureViewDimension::D2,
        //                     sample_type: wgpu::TextureSampleType::Float { filterable: true },
        //                 },
        //                 count: None,
        //             },
        //             wgpu::BindGroupLayoutEntry {
        //                 binding: 1,
        //                 visibility: wgpu::ShaderStages::FRAGMENT,
        //                 // This should match the filterable field of the
        //                 // corresponding Texture entry above.
        //                 ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        //                 count: None,
        //             },
        //         ],
        //         label: Some("texture_bind_group_layout"),
        //     },
        // ));

        // render_stuff.insert({
        //     let view_textures_bind_group_layout =
        //         device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //             entries: &[
        //                 wgpu::BindGroupLayoutEntry {
        //                     binding: 0,
        //                     visibility: wgpu::ShaderStages::FRAGMENT,
        //                     ty: wgpu::BindingType::Texture {
        //                         multisampled: false,
        //                         view_dimension: wgpu::TextureViewDimension::D2,
        //                         sample_type: wgpu::TextureSampleType::Float { filterable: true },
        //                     },
        //                     count: None,
        //                 },
        //                 wgpu::BindGroupLayoutEntry {
        //                     binding: 1,
        //                     visibility: wgpu::ShaderStages::FRAGMENT,
        //                     // This should match the filterable field of the
        //                     // corresponding Texture entry above.
        //                     ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        //                     count: None,
        //                 },
        //                 wgpu::BindGroupLayoutEntry {
        //                     binding: 2,
        //                     visibility: wgpu::ShaderStages::FRAGMENT,
        //                     ty: wgpu::BindingType::Texture {
        //                         multisampled: false,
        //                         view_dimension: wgpu::TextureViewDimension::D2,
        //                         sample_type: wgpu::TextureSampleType::Depth,
        //                     },
        //                     count: None,
        //                 },
        //                 wgpu::BindGroupLayoutEntry {
        //                     binding: 3,
        //                     visibility: wgpu::ShaderStages::FRAGMENT,
        //                     // This should match the filterable field of the
        //                     // corresponding Texture entry above.
        //                     ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        //                     count: None,
        //                 },
        //             ],
        //             label: Some("post_texture_bind_group_layout"),
        //         });

        //     let color_texture = make_view_texture(&config, &device);
        //     let depth_texture =
        //         texture::Texture::create_depth_texture(&device, &config, "depth_texture");
        //     ViewTextureReasources {
        //         view_textures_bind_group: make_view_textures_bind_group(
        //             &device,
        //             &view_textures_bind_group_layout,
        //             &depth_texture,
        //             &color_texture,
        //         ),
        //         view_textures_bind_group_layout,
        //         depth_texture,
        //         view_texture: color_texture,
        //     }
        // });

        // let planet_model =
        //     resources::load_model("wallmart.obj", &device, &queue, &texture_bind_group_layout)
        //         .await
        //         .unwrap();

        // let ship_model =
        //     resources::load_model("retroShip.obj", &device, &queue, &texture_bind_group_layout)
        //         .await
        //         .unwrap();

        // let planet_transform_instance_buffer = Arc::new(Mutex::new(InstanceBuffer::new(
        //     Some("Planet instance buffer".to_owned()),
        //     BufferUsages::VERTEX | BufferUsages::COPY_DST,
        //     &device,
        // )));
        // // let ship_transform_instance_buffer = ;

        // let arrow_model =
        //     resources::load_model("arrow.obj", &device, &queue, &texture_bind_group_layout)
        //         .await
        //         .unwrap();
        // // let arrow_transform_buffer_data = arrows.to_raw_buffer();
        // let arrow_transform_instance_buffer = InstanceBuffer::new(
        //     Some("arrow instance Buffer".to_owned()),
        //     wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        //     &device,
        // );

        // let gpu = Gpu {
        //     device: Arc::new(device),
        //     queue: Arc::new(queue),
        // };
        // render_stuff.insert(gpu.clone());
        // ecs.world.insert_resource(gpu.clone());
        // ecs.world.insert_resource(Camera::default());

        // time::init(&mut ecs).await;
        // physics::init(&mut ecs).await;
        // planets::init(
        //     &mut ecs,
        //     &gpu,
        //     &texture_bind_group_layout,
        //     planet_transform_instance_buffer.clone(),
        // )
        // .await;
        // gravity::init(&mut ecs).await;
        // input::init(&mut ecs);
        // player_controller::init(&mut ecs);
        // gizmo::init(&mut ecs);
        // ship::init(&mut ecs, &mut render_stuff);
        // sun::init(
        //     &mut ecs,
        //     &mut render_stuff,
        //     &mut shader_manager,
        //     &gpu,
        //     &texture_bind_group_layout,
        // )
        // .await;

        // render_stuff.insert(FullscreenQuad::init(&gpu.device, &mut shader_manager));

        // if let Some((vt,)) = get_all!(render_stuff, ViewTextureReasources) {
        //     render_stuff.insert(PlanetShadows::new(
        //         &gpu.device,
        //         &config,
        //         &mut shader_manager,
        //         &vt.view_textures_bind_group_layout,
        //     ));
        // }

        // Background::init(
        //     &gpu.device,
        //     &mut shader_manager,
        //     &mut render_stuff,
        //     &mut ecs,
        // );

        // if let Some((vt, ps, bg, fsq)) = get_all!(
        //     render_stuff,
        //     ViewTextureReasources,
        //     PlanetShadows,
        //     Background,
        //     FullscreenQuad
        // ) {
        //     render_stuff.insert(Postprocessing::new(
        //         &gpu.device,
        //         &config,
        //         &fsq,
        //         &vt.view_textures_bind_group_layout,
        //         &ps.texture_bind_group_layout,
        //         &bg.texture_cube_bind_group_layout,
        //         &mut shader_manager,
        //     ));
        // }
        // BloomReasoruces::init(&gpu, &config, &mut render_stuff, &mut shader_manager);

        // Bloom::init(&config, &gpu.device, &mut render_stuff);

        // init_normal_pipeline(
        //     &mut render_stuff,
        //     &mut shader_manager,
        //     &texture_bind_group_layout,
        // );

        // let gizmo_shader = shader_manager.load_shader("./src/gizmo.wgsl");
        // // let tbgl = texture_bind_group_layout.clone();
        // // let cgs = camera_gpu_stuff.clone();
        // let gizmo_layout = Box::leak(Box::new(mk_pipeline_layout(
        //     &gpu.device.clone(),
        //     "gizmo",
        //     &[&render_stuff
        //         .get::<CameraUniform>()
        //         .unwrap()
        //         .inner
        //         .bind_group_layout],
        // )));
        // let gizmo_render_pipeline = shader_manager.add_render_pipeline(
        //     UnifiedPipelineModuleDescriptor {
        //         handle: gizmo_shader,
        //     },
        //     Box::new(move |_rr, device, module| {
        //         mk_pipeline(
        //             &device,
        //             gizmo_layout,
        //             None,
        //             &module,
        //             "gizmo",
        //             RawArrowInstance::desc(),
        //         )
        //     }),
        // );

        // tri_meash_gen::init(&mut shader_manager, &mut render_stuff, &gpu.device);

        // let tri_meshes = vec![tri_meash_gen::prepare_mesh(
        //     &tri_meash_gen::gen_mesh(100, 100, terain_height),
        //     &gpu,
        // )];

        // Self {
        // planet_model,

        // planets,
        // planet_transform_instance_buffer: planet_transform_instance_buffer.into(),
        // render_world,
        // ship_transforms,
        // ship_model,
        // fire_bg: true,
        // arrows,
        // arrow_transform_instance_buffer,
        // arrow_model,
        // gizmo_render_pipeline,
        // ecs,
        // gilrs: Gilrs::new().unwrap(),
        // shader_manager,
        // render_stuff,
        // tri_meshes,
        // graphics_ctx: None,
        // render_init,
        // shader_manager: ,
        // }

        // tri_meshes: Vec<tri_meash_gen::TriMesh>,

        // fire_bg: bool,
        // // planets: Planets,
        // planet_model: Model,
        // planet_transform_instance_buffer: Arc<Mutex<InstanceBuffer>>,

        // // ship_transforms: Vec<Transform>,
        // ship_model: Model,

        // // arrows: ArrowGizmos,
        // arrow_transform_instance_buffer: InstanceBuffer,
        // arrow_model: Model,
        // gizmo_render_pipeline: PipelineHandle,
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        let Some(mut g_ctx) = self.graphics_ctx else {
            panic!("Graphics context not initialised");
        };
        if new_size.width > 0 && new_size.height > 0 {
            g_ctx.size = new_size;
            g_ctx.config.width = new_size.width;
            {
                let mut camera = self.ecs.world.resource_mut::<Camera>();
                camera.aspect = new_size.width as f32 / new_size.height as f32;
                self.render_stuff
                    .get_mut::<CameraUniform>()
                    .unwrap()
                    .update(&camera, &g_ctx.gpu.queue);
            }
            g_ctx.config.height = new_size.height;
            g_ctx.surface.configure(&g_ctx.gpu.device, &g_ctx.config);

            if let Some(ViewTextureReasources {
                ref mut view_texture,
                ref mut view_textures_bind_group,
                ref mut view_textures_bind_group_layout,
                ref mut depth_texture,
            }) = self.render_stuff.get_mut::<ViewTextureReasources>()
            {
                *depth_texture = texture::Texture::create_depth_texture(
                    &g_ctx.gpu.device,
                    &g_ctx.config,
                    "depth_texture",
                );
                *view_texture = make_view_texture(&g_ctx.config, &g_ctx.gpu.device);
                *view_textures_bind_group = make_view_textures_bind_group(
                    &g_ctx.gpu.device,
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
                        &g_ctx.config,
                        &g_ctx.gpu.device,
                        &view_texture.view,
                        &self.render_stuff,
                    );
                }
                self.render_stuff.insert(bloom);
            }

            if let Some(planet_shadows) = self.render_stuff.get_mut::<PlanetShadows>() {
                planet_shadows.resize(&g_ctx.config, &g_ctx.gpu.device);
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
            WindowEvent::KeyboardInput { event, .. } => {
                if let KeyEvent {
                    state,
                    logical_key: Some(key),
                    ..
                } = event
                {
                    let mut kb: Mut<ButtonInput> = self.ecs.world.resource_mut();
                    match state {
                        ElementState::Pressed => kb.on_key_down(*key),
                        ElementState::Released => kb.on_key_up(*key),
                    };

                    let pressed = *state == ElementState::Pressed;
                    if *key == Key::B {
                        self.fire_bg = true;
                    }

                    false
                } else {
                    false
                }
            }
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
            let Ok(sun_ib) = sun_rr.sun_transform_instance_buffer.lock() else {
                panic!()
            };
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
                render_pass.set_vertex_buffer(1, sun_ib.buffer.slice(..));
                render_pass.set_pipeline(pipeline);

                render_pass.draw_model_instanced(
                    &sun_rr.sun_model,
                    0..sun_ib.instances,
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

#[derive(Resource)]
struct TextureBindGroupLayout(BindGroupLayout);

fn init_normal_pipeline(
    shader_manager: Res<ShaderManagerCtrl>,
    texture_bind_group_layout: &Res<TextureBindGroupLayout>,
    DeviceRes(device): Res<DeviceRes>,
    background: Res<Background>,
    commands: Commands, // let (gpu, background) = get_all!(render_stuff, Gpu, Background).unwrap();
    cam_uniform: Res<CameraUniform>,
) {
    struct NormalPiplineLayout {
        layout: PipelineLayout,
    }
    let normal_shader = shader_manager.load_shader("./src/shader.wgsl");

    commands.insert_resource(NormalPiplineLayout {
        layout: mk_pipeline_layout(
            &device.clone(),
            "normal",
            &[
                cam_uniform.bind_group_layout,
                &texture_bind_group_layout,
                &background.texture_cube_bind_group_layout,
            ],
        ),
    });
    commands.insert_resource(StandardRenderPipeline {
        handle: shader_manager.add_render_pipeline(
            UnifiedPipelineModuleDescriptor {
                handle: normal_shader,
            },
            |In(module): In<UnifiedPipelineModuleResource>,
             NormalPiplineLayout { layout }: Res<NormalPiplineLayout>| {
                mk_pipeline(
                    &device,
                    &layout,
                    None,
                    &module,
                    "normal",
                    RawInstance::desc(),
                )
            },
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
        mipmap_filter: wgpu::FilterMode::Linear,
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
                entry_point: "vs_main", // 1.
                buffers: &[model::ModelVertex::desc(), descritptor],
                compilation_options: Default::default(), // 2.
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
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // cull_mode: None,
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
            multiview: None,
            cache: None, // 5.
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

fn instance_buffer_bytes<'a>(
    items: impl Iterator<Item = &'a Transform>,
    origin: Vec3,
) -> Vec<RawInstance> {
    items
        .map(|t| RawInstance {
            transform: t.translated(-origin).to_mat(),
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
    const ATTRIBS: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
     // Model matrix
     5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4,
     // Model rotation matrix
     9 => Float32x3, 10 => Float32x3, 11 => Float32x3,
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
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

// fn create_instance_buffers<Marker>(
//     q: Query<Transform, With<Marker>>,
//     camera: Res<Camera>
//     buffer: Option<Res<InstanceBuffer<Marker>>>,
//     commands: Commands,
// ) {

//     let Some(instance_buffer) = buffer else {
//         InstanceBuffer::new()
//     };

//     instance_buffer.write(
//         instance_buffer_bytes(q.iter(), camrea.eye)
//     );

//     if buffer.is_none() {
//         commands.insert_resource(instance_buffer);
//     }

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

fn drawObject(v: Vec3) -> f32 {
    let p = (v.fract() - 0.5).abs();
    return p.x.max(p.y).max(p.z);
}

fn cellTile(v: Vec3) -> f32 {
    let mut p = v;
    // Draw four overlapping objects (spheres, in this case) at various positions throughout the tile.
    let mut d = Vec4::ZERO;
    d.x = drawObject(p - vec3(0.81, 0.62, 0.53));
    p.x = (p.y - p.x) * 0.7071;
    p.y = (p.y + p.x) * 0.7071;
    //p.xy = vec2(p.y*.866025 - p.x*.5, p.y*.5 + p.x*.866025); // Etc.
    d.y = drawObject(p - vec3(0.39, 0.2, 0.11));
    p.y = (p.z - p.y) * 0.7071;
    p.z = (p.z + p.y) * 0.7071;
    //p.yz = vec2(p.z*.866025 - p.y*.5, p.z*.5 + p.y*.866025); // Etc.
    d.z = drawObject(p - vec3(0.62, 0.24, 0.06));
    p.x = (p.z - p.x) * 0.7071;
    p.z = (p.z + p.x) * 0.7071;
    //p.xz = vec2(p.z*.866025 - p.x*.5, p.z*.5 + p.x*.866025); // Etc.
    d.w = drawObject(p - vec3(0.2, 0.82, 0.64));

    // Obtain the minimum, and you're done.
    d.x = d.x.min(d.y);
    d.y = d.z.min(d.w);

    return 1. - d.x.min(d.y) * 2.; // Scale between zero and one... roughly.

    // For anyone wanting to experiment with this, the following gives better variance:
    //const float scale = 1.; // 1 up to 4, or higher, depending on the look you want.
    // Obviously, for the reverse, you take the one and minus away.
    //return 1. - min(min(d.x, d.y)*2.*scale, 1.);
}

fn terain_height(p: Vec2) -> f32 {
    let position = vec3(p.x, 0.0, p.y);
    smoothstep(0.5, 1.0, cellTile(position * 0.1)) * 2.0
        + 0.0 * (0.4 + ((position.z * 0.5 + (position.x * 0.5).sin()).sin())).abs()
}

#[inline(always)]
fn smoothstep(min: f32, max: f32, value: f32) -> f32 {
    let x = ((value - min) / (max - min)).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}
