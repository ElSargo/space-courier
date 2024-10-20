use crate::{prelude::*, shader_manager};
use std::sync::atomic::AtomicBool;

#[derive(Resource)]
pub struct Background {
    pub texture: TexCube,
    texture_array_bind_group: wgpu::BindGroup,
    pub texture_cube_bind_group: wgpu::BindGroup,
    pub texture_cube_bind_group_layout: wgpu::BindGroupLayout,
    pipeline: PipelineHandle,
    generating: Arc<AtomicBool>,
}

#[derive(Resource)]
struct ConstructorReasources {
    pipeline_layout: PipelineLayout,
}

static FORMAT: TextureFormat = wgpu::TextureFormat::Rgba16Float;
const DIM: u32 = 1024;
static SIZE: wgpu::Extent3d = wgpu::Extent3d {
    width: DIM,
    height: DIM,
    depth_or_array_layers: 6,
};

pub fn init_mod(ecs: &mut Initailiser) {
    ecs.prepare_buffers.add_systems(update_time_buffer);
    ecs.init.add_systems(init_background);
    ecs.render.add_systems(fire_bg_gen);
    ecs.update.add_systems(keyboard_generate_background);
    ecs.world.init_resource::<Events<FireBGGenEvent>>();
}

pub fn init_background(device: RD, shader_manager: Res<ShaderLoader>, mut commands: Commands) {
    let storage_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Background storage texture layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: FORMAT,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    let cube_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Background storage texture layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let pipline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Background full pipeline layout"),
        bind_group_layouts: &[&storage_layout],
        push_constant_ranges: &[],
    });

    let shader_handle = shader_manager.load_shader("./src/background.wgsl");

    commands.insert_resource(ConstructorReasources {
        pipeline_layout: pipline_layout,
    });

    let pipeline = shader_manager.add_compute_pipeline(
        UnifiedShaders {
            handle: shader_handle,
        },
        background_pipeline_constructor,
    );

    let tex = device.create_texture(&TextureDescriptor {
        label: Some("Background storage"),
        size: SIZE,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: FORMAT,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let view_cube = tex.create_view(&TextureViewDescriptor {
        label: Some("Background texture cube view"),
        dimension: Some(wgpu::TextureViewDimension::Cube),
        aspect: wgpu::TextureAspect::All,
        array_layer_count: Some(6),
        ..Default::default()
    });

    let view_array = tex.create_view(&TextureViewDescriptor {
        label: Some("Background texture cube view"),
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        aspect: wgpu::TextureAspect::All,
        array_layer_count: Some(6),
        ..Default::default()
    });

    let time_buffer = Arc::new(
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Background time buffer"),
            contents: 0.0_f32.to_ne_bytes().as_slice(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        }),
    );

    let texture_array_bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Background storage bind group"),
        layout: &storage_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view_array),
            },
            BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &time_buffer,
                    offset: 0,
                    size: None,
                }),
            },
        ],
    });

    let texture_cube_bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Background cube bind group"),
        layout: &cube_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view_cube),
            },
            BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&device.create_sampler(
                    &wgpu::SamplerDescriptor {
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
                    },
                )),
            },
        ],
    });

    let cube = TexCube { view: view_cube };

    let me = Background {
        texture: cube,
        pipeline,
        texture_array_bind_group,
        texture_cube_bind_group,
        texture_cube_bind_group_layout: cube_layout,
        generating: Arc::new(false.into()),
    };

    commands.insert_resource(me);
    commands.insert_resource(TimeBuffer(time_buffer));
}

#[derive(Event)]
pub struct FireBGGenEvent;

pub fn keyboard_generate_background(
    bg: ResMut<Background>,
    mut fire_bg: EventWriter<FireBGGenEvent>,
    input: Res<ButtonInput>,
) {
    if input.just_down(winit::keyboard::KeyCode::KeyB) {
        let old = bg.generating.compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::Acquire,
            std::sync::atomic::Ordering::Relaxed,
        );
        if old.is_ok() {
            fire_bg.send(FireBGGenEvent);
        }
    }
}

fn fire_bg_gen(
    shader_manager: Res<shader_manager::PipelinesView<wgpu::ComputePipeline>>,
    bg: ResMut<Background>,
    mut cmd_encoder: ResMut<Marker<CommandEncoder, Background>>,
    mut callbacks: ResMut<RenderCompleteCallBack>,
    mut fire_bg: EventReader<FireBGGenEvent>,
) {
    if fire_bg.read().last().is_some() {
        println!("BG gen");
        if let Some(pipeline) = shader_manager.get(bg.pipeline) {
            let mut pass = cmd_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Background pass"),
                ..Default::default()
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bg.texture_array_bind_group, &[]);
            pass.dispatch_workgroups(DIM / 16, DIM / 16, 6);
        }

        let v = bg.generating.clone();
        callbacks.push(Box::new(move || v.store(false, Ordering::SeqCst)));
    }
}

fn background_pipeline_constructor(
    In(module): In<UnifiedPipelineModuleResource>,
    device: RD,
    layout: Res<ConstructorReasources>,
    mut fire_bg: EventWriter<FireBGGenEvent>,
) -> wgpu::ComputePipeline {
    fire_bg.send(FireBGGenEvent);
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Background pipeline"),
        layout: Some(&layout.pipeline_layout),
        module: &module,
        entry_point: "full",
        cache: None,
        compilation_options: Default::default(),
    })
}

#[derive(Resource)]
struct TimeBuffer(Arc<wgpu::Buffer>);
fn update_time_buffer(buf: Res<TimeBuffer>, queue: RQ, time: Res<Time>) {
    queue.write_buffer(
        &buf.as_ref().0,
        0,
        (time.since_startup as f32).to_ne_bytes().as_slice(),
    );
}
