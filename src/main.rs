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
mod marker;
pub mod model;
pub mod model_loader;
mod physics;
mod planet_shadows;
mod planets;
mod player_controller;
mod post;
mod prelude;
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
pub use crate::{
    bloom::Bloom,
    ecs::{PrepareBuffersSchedual, RenderSchedual, UpdateSchedual},
    gpu::ScreenFormat,
    input::AxisEvent,
    post::Postprocessing,
    solid_pass::Solidpass,
};
use gpu::{DeviceResource, OutputTextureResource};
use prelude::*;

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

use tri_meash_gen::positon_to_chunk_id;
use wgpu::PresentMode;
// lib.rs
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey, PhysicalKey},
    window::{Window, WindowAttributes},
};

struct GraphicsContext<'a> {
    surface: wgpu::Surface<'a>,
    window: Arc<Window>,
    device: DeviceResource,
    queue: QueueResource,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    _adapter: wgpu::Adapter,
    shader_manager: ShaderManager,
}

impl GraphicsContext<'_> {
    fn new(event_loop: &ActiveEventLoop, world: &mut World) -> Self {
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

        // TODO other output formats
        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_capabilities.formats[0]);
        let size = window.inner_size();

        // TODO proper vsync settings
        let present_mode = if surface_capabilities
            .present_modes
            .contains(&PresentMode::Fifo)
        {
            // Get vsync if we can
            PresentMode::Fifo
        } else {
            // Whatever
            surface_capabilities.present_modes[0]
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
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
        let shader_manager = ShaderManager::new(device_res, world).unwrap();

        world.insert_resource(device.clone());
        world.insert_resource(queue.clone());
        world.insert_resource(ScreenResolution {
            width: config.width,
            height: config.height,
        });
        world.insert_resource(ScreenFormat {
            format: config.format,
        });

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            _adapter: adapter,
            shader_manager,
        }
    }
}

struct State<'a> {
    graphics_ctx: Option<GraphicsContext<'a>>,
    initialiser: Option<Initailiser>,
    world: Option<World>,
    key_events: VecDeque<KeyEvent>,
}

impl<'a> ApplicationHandler for State<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("Resumed");
        if self.graphics_ctx.is_none() {
            let mut initialiser = self.initialiser.take().unwrap();
            init_modules(&mut initialiser);

            let mut world = initialiser.finalize();
            let gtx = GraphicsContext::new(event_loop, &mut world);

            world.run_schedule(InitSchedual);

            self.world = Some(world);
            self.graphics_ctx = Some(gtx);
        }
        // self.window().request_redraw();
        println!("Resumed complete");
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        match event {
            winit::event::DeviceEvent::Motion { axis, value } => {
                if let Some(ref mut world) = self.world {
                    world.send_event(AxisEvent {
                        axis,
                        value: value as f32,
                    });
                }
            }
            _ => {}
        };
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
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
                            self.resize(physical_size);
                            self.window().request_redraw();
                        },
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                                winit::event::KeyEvent {
                                    logical_key: Key::Named(NamedKey::Escape),
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
        if let Some((g_ctx, world)) = self.graphics_ctx.as_mut().zip(self.world.as_mut()) {
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

        {
            let mut queue = world.resource_mut::<RenderCompleteCallBack>();
            while let Some(cb) = queue.pop() {
                g_ctx.queue.on_submitted_work_done(cb);
            }
        }

        g_ctx
            .queue
            .submit(encoders.into_iter().map(|encoder| encoder.finish()));
        output.present();

        Ok(())
    }
}

fn main() {
    color_eyre::install().unwrap();
    run();
}

fn log_time(
    mut current_second: Local<f64>,
    mut frame: Local<u128>,
    mut avg: Local<f64>,
    time: Res<Time>,
    camera: Res<Camera>,
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

        println!("Cam_chunk_id: {:?}", positon_to_chunk_id(camera.eye.xz()))
    }

    if time.delta_seconds > 1.0 / 40.0 {
        println!(
            "[Time] {}: {} millis",
            "Lag spike".red(),
            time.delta_seconds * 1000.0
        );
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
            $world.insert_resource::<crate::marker::Marker<wgpu::CommandEncoder, $id>>(
                crate::marker::Marker::new(encoder),
            );

        )*

        $run

        [
            $(
              $world.remove_resource::<crate::marker::Marker<wgpu::CommandEncoder, $id>>().unwrap().into_inner(),
            )*
        ]
        }
    };
}
#[macro_export]
macro_rules! standard_object {
    ($ecs: expr, $renderable: expr, $marker: expr, $input_filter: ty, $buffer_filter: ty, $pipeline: tt) => {{
        use crate::{
            gpu::TextureBindGroupLayoutResource, model_loader, prelude::*,
            renderables::SolidRenderable,
        };

        fn init(
            device: RD,
            queue: RQ,
            tr: Res<TextureBindGroupLayoutResource>,
            shader_manager: Res<ShaderLoader>,
            mut commands: Commands,
        ) {
            commands.spawn((
                SolidRenderable {
                    renderable: Box::new(model_loader::load_model(
                        $renderable,
                        device.0.clone(),
                        queue.0.clone(),
                        tr.0.clone(),
                    )),
                    pipeline: $pipeline(shader_manager),
                },
                $marker,
                InstanceBuffer::new(
                    Some(concat![stringify!($buffer_filter), " instance Buffer"].to_owned()),
                    wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    &device,
                ),
            ));
        }
        $ecs.prepare_buffers
            .add_systems(pack_buffers::<RawInstance, $input_filter, $buffer_filter>);
        $ecs.init.add_systems(
            init.after(crate::background::init_background)
                .after(crate::solid_pass::init_normal_pipeline),
        );
    }};

    ($ecs: expr, $renderable: expr, $marker: ident, $input_filter: ty) => {{
        use crate::{
            gpu::TextureBindGroupLayoutResource, instance_buffer::InstanceBufferMarker,
            model_loader, prelude::*, renderables::SolidRenderable, StandardRenderPipeline,
        };
        use std::marker::PhantomData;

        pub fn init(
            device: RD,
            queue: RQ,
            tr: Res<TextureBindGroupLayoutResource>,
            pipeline: Res<StandardRenderPipeline>,
            mut commands: Commands,
        ) {
            commands.spawn((
                SolidRenderable {
                    renderable: Box::new(model_loader::load_model(
                        $renderable,
                        device.0.clone(),
                        queue.0.clone(),
                        tr.0.clone(),
                    )),
                    pipeline: pipeline.handle,
                },
                InstanceBufferMarker(PhantomData::<$marker>),
                InstanceBuffer::new(
                    Some(concat![stringify!($buffer_filter), " instance Buffer"].to_owned()),
                    wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    &device,
                ),
            ));
        }
        $ecs.prepare_buffers.add_systems(
            pack_buffers::<RawInstance, $input_filter, With<InstanceBufferMarker<$marker>>>,
        );
        $ecs.init.add_systems(
            init.after(crate::background::init_background)
                .after(crate::solid_pass::init_normal_pipeline),
        );
    }};
}
