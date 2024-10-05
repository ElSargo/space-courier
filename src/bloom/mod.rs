use bevy_ecs::{
    schedule::IntoSystemConfigs,
    system::{Commands, In, Res, ResMut, Resource},
};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BufferBinding, BufferDescriptor, BufferUsages, Color, CommandEncoder,
    Device, PipelineLayout, RenderPipeline, ShaderModule, ShaderStages, TextureDescriptor,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor, VertexBufferLayout,
};

use crate::{
    background::{self, Background},
    camera::CameraUniform,
    ecs::Initailiser,
    fullscreen::FullscreenQuad,
    gpu::{QueueResource, ScreenResolution, RD},
    input::ButtonInput,
    load_shader,
    shader_manager::{
        Marker, PipelineHandle, PipelinesView, ShaderManagerCtrl, SplitPipelineModuleResource,
        SplitShaders,
    },
    ViewTextures,
};

pub fn init_mod(ecs: &mut Initailiser) {
    ecs.init.add_systems(
        init_res
            .after(crate::fullscreen::init_fsq)
            .after(background::init_background)
            .after(create_extract_bind_group),
    );
    ecs.update.add_systems(change_result_texture);
    ecs.render.add_systems(draw_bloom);
    ecs.resize.add_systems(
        (create_extract_bind_group, resize_bloom)
            .chain()
            .after(crate::camera::resize_view_textures),
    );
}

#[derive(Resource)]
pub struct BloomReasoruces {
    down_pipeline: PipelineHandle,
    up_pipeline: PipelineHandle,
    extract_pipeline: PipelineHandle,
    extract_bind_group_layout: wgpu::BindGroupLayout,
    threshold_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,
    sampler_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

#[derive(Default)]
pub struct Chain {
    textures: Vec<wgpu::Texture>,
    pub texture_views: Vec<wgpu::TextureView>,
    bind_groups: Vec<wgpu::BindGroup>,
}

impl Chain {
    fn empty(&mut self) {
        self.textures.clear();
        self.texture_views.clear();
        self.bind_groups.clear();
    }
}

#[derive(Resource)]
pub struct Bloom {
    down_chain: Chain,
    pub up_chain: Chain,
    extract_texture: wgpu::Texture,
    extract_texture_view: wgpu::TextureView,
    extract_bindings: wgpu::BindGroup,
    which_result: (bool, usize),
}

impl BloomReasoruces {
    fn update_threshold(&self, queue: &wgpu::Queue, threshold: f32) {
        queue.write_buffer(&self.threshold_buffer, 0, &threshold.to_ne_bytes());
    }
}

impl Bloom {
    pub fn get_result_bind_group(&self) -> &BindGroup {
        if self.which_result.0 {
            &self.up_chain.bind_groups[self.up_chain.textures.len() - self.which_result.1 - 1]
        } else {
            &self.down_chain.bind_groups[self.which_result.1]
        }
    }
}

fn mk_bind_group(
    device: &Device,
    br: &BloomReasoruces,
    previous_view: &TextureView,
    width: u32,
    height: u32,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        label: Some(format!("Bloom down chain bind group {width} {height}").as_str()),
        layout: &br.texture_bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(previous_view),
        }],
    })
}

fn create_texture(device: &Device, size: (u32, u32)) -> wgpu::Texture {
    device.create_texture(&TextureDescriptor {
        label: Some("Bloom extrsact texture"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        view_formats: &[],
    })
}

fn create_extract_bind_group(
    device: RD,
    vtr: Res<ViewTextures>,
    br: Res<BloomReasoruces>,
    bg: Res<Background>,
    cam: Res<CameraUniform>,
    mut bloom: ResMut<Bloom>,
) {
    bloom.extract_bindings = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Bloom extract bind group"),
        layout: &br.extract_bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Sampler(&br.sampler),
            },
            BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &br.threshold_buffer,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&bg.texture.view),
            },
            BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&vtr.color_texture.view),
            },
            BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &cam.buffer,
                    offset: 0,
                    size: None,
                }),
            },
        ],
    });
}

fn extract_bind_group_layout(device: &Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        // @group(0) @binding(0) var color_sampler: sampler;
        // @group(0) @binding(1) var<uniform> threshold: f32;
        // @group(0) @binding(2) var background: texture_cube<f32>;
        // @group(0) @binding(3) var color_texture: texture_2d<f32>;
        // @group(0) @binding(4) var<uniform> camera: Camera;
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 4,
                visibility: ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
        label: Some("Bloom extract bind group lauout"),
    })
}

fn mk_pipeline(
    label: &'static str,
    device: &Device,
    layout: &PipelineLayout,
    vertex_layout: &'static [VertexBufferLayout<'static>],
    fullscreen_shader: &ShaderModule,
    bloom_shader: &ShaderModule,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: fullscreen_shader,
            entry_point: "vs_main",
            buffers: vertex_layout,
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            // 3.
            compilation_options: Default::default(),
            module: &bloom_shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                // 4.
                format: wgpu::TextureFormat::Rgba16Float,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList, // 1.
            strip_index_format: None,
            front_face: wgpu::FrontFace::Cw, // 2.
            // cull_mode: Some(wgpu::Face::Back),
            cull_mode: None,
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: None, // 1.
        multisample: wgpu::MultisampleState {
            count: 1,                         // 2.
            mask: !0,                         // 3.
            alpha_to_coverage_enabled: false, // 4.
        },
        multiview: None,
        cache: None, // 5.
    })
}

pub fn draw_bloom(
    render_pipelines: Res<PipelinesView<RenderPipeline>>,
    mut encoder: ResMut<Marker<CommandEncoder, Bloom>>,
    bloom: Res<Bloom>,
    br: Res<BloomReasoruces>,
    fullscreen_quad: Res<FullscreenQuad>,
) {
    if let Some(pipeline) = render_pipelines.get(&br.extract_pipeline) {
        // println!("bloom extract drawing");
        // for i in 0..bloom.down_chain.textures.len() {
        // println!("Down: {:#?}", bloom.down_textures[i].size());
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Bloom extract render pass {i}"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &bloom.extract_texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_bind_group(0, &bloom.extract_bindings, &[]);
        render_pass.set_pipeline(&pipeline);
        fullscreen_quad.draw(&mut render_pass);
        // }
    } else {
        println!("Bloom extract not drawn");
    }

    if let Some(pipeline) = render_pipelines.get(&br.down_pipeline) {
        // println!("bloom down drawing");
        for i in 0..bloom.down_chain.textures.len() {
            // println!("Down: {:#?}", bloom.down_textures[i].size());
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Bloom down render pass {i}"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &bloom.down_chain.texture_views[i],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_bind_group(0, &br.sampler_bind_group, &[]);
            render_pass.set_bind_group(1, &bloom.down_chain.bind_groups[i], &[]);
            render_pass.set_pipeline(&pipeline);
            fullscreen_quad.draw(&mut render_pass);
        }
    } else {
        println!("Bloom down not drawn");
    }

    //up  :0 1 2 3 4 5
    //down:0 1 2 3 4 5

    if let Some(pipeline) = render_pipelines.get(&br.up_pipeline) {
        // println!("bloom up drawing");
        for i in 1..bloom.up_chain.textures.len() {
            let view = &bloom.up_chain.texture_views[i];
            let down_texture =
                &bloom.down_chain.bind_groups[bloom.down_chain.bind_groups.len() - 1 - i];
            let prev_texture = &bloom.up_chain.bind_groups[i - 1];
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Bloom render pass {i}"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_bind_group(0, &br.sampler_bind_group, &[]);
            render_pass.set_bind_group(1, &down_texture, &[]);
            render_pass.set_bind_group(2, &prev_texture, &[]);
            render_pass.set_pipeline(&pipeline);
            fullscreen_quad.draw(&mut render_pass);
        }
    } else {
        println!("Bloom down not drawn");
    }
}

pub fn init_res(
    device: RD,
    queue: Res<QueueResource>,
    fsq: Res<FullscreenQuad>,

    mut commands: Commands,
    shader_manager: Res<ShaderManagerCtrl>,
) {
    // let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
    //     label: Some("Bloom bind group layout"),
    //     entries: &[
    //         BindGroupLayoutEntry {
    //             binding: 0,
    //             visibility: ShaderStages::FRAGMENT,
    //             ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
    //             count: None,
    //         },
    //         BindGroupLayoutEntry {
    //             binding: 1,
    //             visibility: ShaderStages::FRAGMENT,
    //             ty: wgpu::BindingType::Texture {
    //                 sample_type: wgpu::TextureSampleType::Float { filterable: true },
    //                 view_dimension: wgpu::TextureViewDimension::D2,
    //                 multisampled: false,
    //             },
    //             count: None,
    //         },
    //         BindGroupLayoutEntry {
    //             binding: 2,
    //             visibility: ShaderStages::FRAGMENT,
    //             ty: wgpu::BindingType::Texture {
    //                 sample_type: wgpu::TextureSampleType::Float { filterable: true },
    //                 view_dimension: wgpu::TextureViewDimension::D2,
    //                 multisampled: false,
    //             },
    //             count: None,
    //         },
    //     ],
    // });

    // struct BloomPipelineLayout {
    //     layout: PipelineLayout,
    // }

    // let (Background {
    //     texture_cube_bind_group_layout,
    //     ..
    // },) = get_all!(rr, Background).unwrap();
    // let blay = BloomPipelineLayout {
    //     layout: gpu
    //         .device
    //         .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    //             label: Some("bloom pipline layout"),
    //             bind_group_layouts: &[&bind_group_layout],
    //             push_constant_ranges: &[],
    //         }),
    // };

    // rr.insert(blay);

    let sampler_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Bloom sampler bind group layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        }],
    });

    let texture_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Bloom texture bind group layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }],
    });

    let down_pipeline = {
        let bloom_down_handle = load_shader!(shader_manager, "bloom_down.wgsl");
        #[derive(Resource)]
        struct DownPipelineLayout(PipelineLayout);
        commands.insert_resource(DownPipelineLayout(device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Bloom down render pipeline layout"),
                bind_group_layouts: &[&sampler_bind_group_layout, &texture_bind_group_layout],
                push_constant_ranges: &[],
            },
        )));
        shader_manager.add_render_pipeline(
            SplitShaders {
                vertex: fsq.shader_handle,
                fragment: bloom_down_handle,
            },
            move |In(modules): In<SplitPipelineModuleResource>,
                  device: RD,
                  fsq: Res<FullscreenQuad>,
                  dpl: Res<DownPipelineLayout>| {
                mk_pipeline(
                    "Bloom down render pipeline",
                    &device,
                    &dpl.0,
                    fsq.vertex_format,
                    &modules.vertex,
                    &modules.fragment,
                )
            },
        )
    };
    let up_pipeline = {
        let bloom_up_handle = load_shader!(shader_manager, "bloom_up.wgsl");
        #[derive(Resource)]
        struct UpPipelineLayout(PipelineLayout);
        commands.insert_resource(UpPipelineLayout(device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Bloom up render pipeline layout"),
                bind_group_layouts: &[
                    &sampler_bind_group_layout,
                    &texture_bind_group_layout,
                    &texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            },
        )));

        shader_manager.add_render_pipeline(
            SplitShaders {
                vertex: fsq.shader_handle,
                fragment: bloom_up_handle,
            },
            |In(SplitPipelineModuleResource { vertex, fragment }),
             device: RD,
             fsq: Res<FullscreenQuad>,
             upl: Res<UpPipelineLayout>| {
                mk_pipeline(
                    "Bloom up render pipeline",
                    &device,
                    &upl.0,
                    fsq.vertex_format,
                    &vertex,
                    &fragment,
                )
            },
        )
    };
    let extract_bind_group_layout = extract_bind_group_layout(&device);
    let bloom_extract_handle = load_shader!(shader_manager, "bloom_extract.wgsl");
    #[derive(Resource)]
    struct ExtractPipelineLayout(PipelineLayout);
    commands.insert_resource(ExtractPipelineLayout(device.create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some("Bloom extract render pipeline layout"),
            bind_group_layouts: &[&extract_bind_group_layout],
            push_constant_ranges: &[],
        },
    )));

    let extract_pipeline = shader_manager.add_render_pipeline(
        SplitShaders {
            vertex: fsq.shader_handle,
            fragment: bloom_extract_handle,
        },
        |In(SplitPipelineModuleResource { vertex, fragment }),
         fsq: Res<FullscreenQuad>,
         epl: Res<ExtractPipelineLayout>,
         device: RD| {
            // Box::new(move |rr, device, modules| {
            //     let (fsq, epl) = get_all!(rr, FullscreenQuad, ExtractPipelineLayout).unwrap();

            mk_pipeline(
                "Bloom extract render pipeline",
                &device,
                &epl.0,
                fsq.vertex_format,
                &vertex,
                &fragment,
            )
        },
    );

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(&format!("Bloom sampler")),
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

    let sampler_bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Bloom sampler bind group"),
        layout: &sampler_bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Sampler(&sampler),
        }],
    });

    let threshold_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("Threshold buffer"),
        size: std::mem::size_of::<f32>() as u64,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bloom_reasources = BloomReasoruces {
        threshold_buffer,
        down_pipeline,
        up_pipeline,
        sampler,
        sampler_bind_group,
        texture_bind_group_layout,
        extract_pipeline,
        extract_bind_group_layout,
    };

    let tmp_tex = create_texture(&device, (1, 1));
    let tmp_v = tmp_tex.create_view(&TextureViewDescriptor {
        label: Some("Dummy blm tex view"),
        format: None,
        dimension: None,
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
    });
    let bloom = Bloom {
        down_chain: Chain::default(),
        up_chain: Chain::default(),
        extract_texture: tmp_tex,
        extract_texture_view: tmp_v,
        extract_bindings: device.create_bind_group(&BindGroupDescriptor {
            label: Some("Dummy, todo remove me"),
            layout: &device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Dummy todo remove me"),
                entries: &[],
            }),
            entries: &[],
        }),
        which_result: (true, 0),
    };

    commands.insert_resource(bloom);

    let threshold: f32 = 2.0;
    bloom_reasources.update_threshold(&queue, threshold);
    commands.insert_resource(bloom_reasources);
}

fn resize_bloom(
    mut bloom: ResMut<Bloom>,
    resolution: Res<ScreenResolution>,
    device: RD,
    br: Res<BloomReasoruces>,
) {
    bloom.down_chain.empty();
    bloom.up_chain.empty();

    // extract_texture: wgpu::Texture,
    bloom.extract_texture = create_texture(&device, (resolution.width, resolution.height));
    bloom.extract_texture_view = bloom.extract_texture.create_view(&TextureViewDescriptor {
        label: Some("Bloom extract texture view"),
        format: Some(TextureFormat::Rgba16Float),
        dimension: Some(wgpu::TextureViewDimension::D2),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: Some(1),
        base_array_layer: 0,
        array_layer_count: Some(1),
    });
    // extract_bindings: wgpu::BindGroup,
    // bloom.extract_bindings is set in create_extract_bind_group

    // down_chain: chain,
    {
        let lowest_x = 20;
        let mut width = resolution.width;
        let mut height = resolution.height;

        let tex = bloom
            .extract_texture
            .create_view(&TextureViewDescriptor::default());
        bloom
            .down_chain
            .bind_groups
            .push(mk_bind_group(&device, &br, &tex, width, height));

        while width > lowest_x {
            width /= 2;
            height /= 2;

            let tex = create_texture(&device, (width, height));
            let view = tex.create_view(&TextureViewDescriptor::default());
            let bind_group = mk_bind_group(&device, &br, &view, width, height);

            bloom.down_chain.textures.push(tex);
            bloom.down_chain.texture_views.push(view);
            bloom.down_chain.bind_groups.push(bind_group);
        }

        for _ in 0..bloom.down_chain.bind_groups.len() {
            let tex = create_texture(&device, (width, height));
            let view = tex.create_view(&TextureViewDescriptor::default());
            let bind_group = mk_bind_group(&device, &br, &view, width, height);

            bloom.up_chain.textures.push(tex);
            bloom.up_chain.texture_views.push(view);
            bloom.up_chain.bind_groups.push(bind_group);

            width *= 2;
            height *= 2;
        }

        println!(
            "Up: {}, Down {}",
            bloom.up_chain.textures.len(),
            bloom.down_chain.textures.len()
        )
    }
}

fn change_result_texture(mut this: ResMut<Bloom>, kb: Res<ButtonInput>) {
    if kb.just_down(winit::keyboard::KeyCode::Equal) {
        this.which_result.1 = (this.which_result.1 + 1).min(this.down_chain.textures.len());
    }
    if kb.just_down(winit::keyboard::KeyCode::Minus) {
        this.which_result.1 = this.which_result.1.saturating_sub(1);
    }
    if kb.just_down(winit::keyboard::KeyCode::Digit0) {
        this.which_result.0 = !this.which_result.0;
    }
}
