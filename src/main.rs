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
mod instance_buffer;
mod model;
mod model_loader;
mod physics;
mod planet_shadows;
mod planets;
mod player_controller;
mod post;
mod renderables;
mod resources;
mod shader_manager;
mod ship;
mod solid_pass;
mod sun;
mod tetrahedron;
mod texture;
mod time;
mod transform;
mod tri_meash_gen;
use crate::{
    bloom::Bloom,
    ecs::{PrepareBuffersSchedual, RenderSchedual, UpdateSchedual},
    gpu::ScreenFormat,
    input::AxisEvent,
    post::Postprocessing,
    solid_pass::Solidpass,
};
use background::Background;
use bevy_ecs::{
    prelude::Resource,
    system::{Local, Res},
    world::World,
};
use ecs::{InitSchedual, Initailiser, ResizeSchedual};
use glam::{Mat3, Mat4, Quat, Vec3};
use gpu::{
    DeviceResource, OutputTextureResource, QueueResource, RenderCompleteCallBack, ScreenResolution,
};
use input::KeyEvent;
use pollster::FutureExt;
use shader_manager::{PipelineHandle, PipelinesView, ShaderManager, ShaderManagerCtrl};
use std::{collections::VecDeque, sync::Arc};
use texture::Texture;
use time::Time;
use transform::Transform;
use wgpu::{
    BindGroup, BindGroupLayout, Device, PipelineLayout, ShaderModule, TextureFormat,
    VertexBufferLayout,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey, PhysicalKey},
    window::WindowAttributes,
};

fn init_modules(initialiser: &mut Initailiser) {
    [
        gpu::init_mod,
        input::init_mod,
        planet_shadows::init_mod,
        background::init_mod,
        planets::init_mod,
        fullscreen::init_mod,
        camera::init_mod,
        ship::init_mod,
        time::init_mod,
        player_controller::init_mod,
        sun::init_mod,
        bloom::init_mod,
        post::init_mod,
        gravity::init_mod,
        solid_pass::init_mod,
        tri_meash_gen::init_mod,
        physics::init_mod,
        tetrahedron::init_mod,
    ]
    .iter()
    .for_each(|f| f(initialiser));
}

#[derive(Resource)]
struct StandardRenderPipeline {
    handle: PipelineHandle,
}

pub fn run() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut state = State::new().block_on();

    event_loop.run_app(&mut state).unwrap();
}

// lib.rs
use winit::window::Window;

#[derive(Resource)]
struct ViewTextures {
    color_texture: Texture,
    depth_texture: Texture,
    bind_group: BindGroup,
}

#[derive(Resource)]
struct ReflectedViewTextures {
    color_texture: Texture,
    depth_texture: Texture,
    bind_group: BindGroup,
    null_texture: Texture,
    null_bind_group: BindGroup,
}

struct GraphicsContext<'a> {
    surface: wgpu::Surface<'a>,
    window: Arc<Window>,
    device: DeviceResource,
    queue: QueueResource,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    adapter: wgpu::Adapter,
    shader_manager: ShaderManager,
    shader_manager_ctl: ShaderManagerCtrl,
    render_pipelines_view: PipelinesView<wgpu::RenderPipeline>,
    compute_pipelines_view: PipelinesView<wgpu::ComputePipeline>,
}

impl GraphicsContext<'_> {
    fn new(event_loop: &ActiveEventLoop) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default().with_inner_size(PhysicalSize::new(1920, 1080)),
                )
                .unwrap(),
        );

        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window, so this should be safe.
        let surface = instance.create_surface(window.clone()).unwrap();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .block_on()
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
        let size = window.inner_size();
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
        limits.max_push_constant_size = 128;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::all_webgpu_mask()
                        | wgpu::Features::PUSH_CONSTANTS,
                    required_limits: limits,
                    label: None,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None, // Trace path
            )
            .block_on()
            .unwrap();
        surface.configure(&device, &config);

        let device_res: Arc<Device> = device.into();
        let device = DeviceResource(device_res.clone());
        let queue = QueueResource(queue.into());
        let (shader_manager, shader_manager_ctl, render_pipelines_view, compute_pipelines_view) =
            ShaderManager::new(device_res).unwrap();

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            adapter,
            shader_manager,
            shader_manager_ctl,
            render_pipelines_view,
            compute_pipelines_view,
        }
    }
}

struct State<'a> {
    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    graphics_ctx: Option<GraphicsContext<'a>>,
    initialiser: Option<Initailiser>,
    world: Option<World>,
    key_events: VecDeque<KeyEvent>,
}

impl<'a> ApplicationHandler for State<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("Resumed");
        if self.graphics_ctx.is_none() {
            let gtx = GraphicsContext::new(event_loop);

            let mut initialiser = self.initialiser.take().unwrap();
            init_modules(&mut initialiser);

            let mut world = initialiser.finalize();

            world.insert_resource(gtx.shader_manager_ctl.clone());
            world.insert_resource(gtx.compute_pipelines_view.clone());
            world.insert_resource(gtx.render_pipelines_view.clone());
            world.insert_resource(gtx.device.clone());
            world.insert_resource(gtx.queue.clone());
            world.insert_resource(ScreenResolution {
                width: gtx.config.width,
                height: gtx.config.height,
            });
            world.insert_resource(ScreenFormat {
                format: gtx.config.format,
            });

            world.run_schedule(InitSchedual);
            world.increment_change_tick();

            self.world = Some(world);
            self.graphics_ctx = Some(gtx);
        }
        // self.window().request_redraw();
        println!("Resumed complete");
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        match event {
            // winit::event::DeviceEvent::MouseMotion { delta } => {
            //     if let Some(ref mut world) = self.world {
            //         world.send_event(MouseDelta {
            //             x: delta.0 as f32,
            //             y: delta.1 as f32,
            //         });
            //     }
            // }
            winit::event::DeviceEvent::Motion { axis, value } => {
                if let Some(ref mut world) = self.world {
                    world.send_event(AxisEvent {
                        axis,
                        value: value as f32,
                    });
                }
            }
            // update
            _ => {} // DeviceEvent::MouseWheel { delta } => todo!(),
                    // DeviceEvent::Button { button, self } => todo!(),
                    // DeviceEvent::Key(e) => self.input(&e),
                    // DeviceEvent::Text { codepoint } => todo!(),
        };
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::RedrawRequested/*(window_id) if window_id == self.window().id() */=> {
                self.update();
                match self.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => self.resize(self.graphics_ctx.as_ref().unwrap().size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
                self.window().request_redraw();
            },
            WindowEvent::Resized(physical_size) => {
                            // RedrawRequested will only trigger once unless we manually
                            // request it.
                            // self.window().request_redraw();
                            // }
                            self.resize(physical_size);
                            self.window().request_redraw();
                        },
                        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                            // new_inner_size is &&mut so we have to dereference it twice
                            // self.resize(**new_inner_size);
                        },
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                                winit::event::KeyEvent {
                                    logical_key: Key::Named(NamedKey::Escape),
                                    state: winit::event::ElementState::Pressed,
                                    ..
                                },
                            ..
                        } => event_loop.exit(),
                        WindowEvent::KeyboardInput {  event: winit::event::KeyEvent { physical_key: PhysicalKey::Code(code), state,.. },..} => {self.key_events.push_back( KeyEvent{ code, pressed: state.is_pressed()} );}
                        WindowEvent::Focused(has_focus) => {
                            let window = &self.graphics_ctx.as_mut().unwrap().window;
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

        // todo!()
    }
}

impl<'a> State<'a> {
    // Creating some of the wgpu types requires async code
    async fn new() -> Self {
        let mut initialiser = Initailiser::new();
        initialiser.update.add_systems(log_time);

        Self {
            graphics_ctx: None,
            initialiser: Some(initialiser),
            world: None,
            key_events: VecDeque::new(),
        }
    }

    pub fn window(&self) -> &Window {
        &self.graphics_ctx.as_ref().unwrap().window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        let Some(ref mut g_ctx) = self.graphics_ctx else {
            panic!("Graphics context not initialised");
        };
        if new_size.width > 0 && new_size.height > 0 {
            g_ctx.size = new_size;
            g_ctx.config.width = new_size.width;
            g_ctx.config.height = new_size.height;
            g_ctx.surface.configure(&g_ctx.device, &g_ctx.config);

            let Some(ref mut world) = self.world else {
                panic!("Render world not initailised");
            };

            {
                let mut res = world.resource_mut::<ScreenResolution>();
                res.width = new_size.width;
                res.height = new_size.height;
            }

            println!("Resize sch running");
            world.run_schedule(ResizeSchedual);
        }
    }

    fn update(&mut self) {
        // println!()
        // assert!(self.graphics_ctx.is_some());
        // assert!(self.world.is_some());
        if let Some((g_ctx, world)) = self.graphics_ctx.as_mut().zip(self.world.as_mut()) {
            let tick = world.resource::<Time>().tick + 1;

            // world.resource_mut::<AxisInput>().update(&mut self.gilrs);

            while let Some(key_event) = self.key_events.pop_front() {
                world.send_event(key_event);
            }

            g_ctx.shader_manager.construct_piplines(world);

            world.run_schedule(UpdateSchedual);
            world.run_schedule(PrepareBuffersSchedual);
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // println!("Render called");

        let Some((g_ctx, world)) = self.graphics_ctx.as_mut().zip(self.world.as_mut()) else {
            panic!("Graphics context or world not initailed");
        };

        let output = g_ctx.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        world.insert_resource(OutputTextureResource(output_view.into()));

        let encoders = insert_run_and_retrive_marked_encoders!(
            world,
            g_ctx.device,
            {
                world.run_schedule(RenderSchedual);
            },
            Background,
            Solidpass,
            Bloom,
            Postprocessing
        );

        g_ctx.device.poll(wgpu::MaintainBase::Wait);

        while let Some(cb) = world.resource_mut::<RenderCompleteCallBack>().pop() {
            g_ctx.queue.on_submitted_work_done(cb);
        }

        g_ctx
            .queue
            .submit(encoders.into_iter().map(|encoder| encoder.finish()));
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

fn make_view_texture(resolution: &ScreenResolution, device: &wgpu::Device) -> Texture {
    let size = wgpu::Extent3d {
        // 2.
        width: resolution.width,
        height: resolution.height,
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

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RawInstance {
    transform: Mat4,
    rotation_only: Mat3,
}
unsafe impl bytemuck::Pod for RawInstance {}
unsafe impl bytemuck::Zeroable for RawInstance {}
instance_layout!(RawInstance, [
     // Model matrix
     5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4,
     // Model rotation matrix
     9 => Float32x3, 10 => Float32x3, 11 => Float32x3,
]);

impl RawInstance {
    pub fn from_transform(t: &Transform) -> Self {
        Self {
            transform: t.to_mat(),
            rotation_only: Mat3::from_quat(t.rotation),
        }
    }
}

fn main() {
    // fn test(x: Vec2) -> f32 {
    //     x.x.sin() * x.y.cos()
    // }
    // let mesh = tri_meash_gen::gen_mesh(5, 5, test);
    // println!("{:#?}", mesh);

    color_eyre::install().unwrap();
    run();
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

#[macro_export]
macro_rules! insert_run_and_retrive_marked_encoders {
    ($world: expr, $dev: expr , $run: tt, $( $id:ident ),*  ) => {
        {

        $(
            let encoder = $dev.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(concat!(stringify!($id), " command encoder")),
            });
            $world.insert_resource::<crate::shader_manager::Marker<wgpu::CommandEncoder, $id>>(
                crate::shader_manager::Marker::new(encoder),
            );

        )*

        $run

        [
            $(
              $world.remove_resource::<crate::shader_manager::Marker<wgpu::CommandEncoder, $id>>().unwrap().into_inner(),
            )*
        ]
        }
    };
}
