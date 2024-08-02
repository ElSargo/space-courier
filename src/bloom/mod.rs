use anymap::AnyMap;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BufferBinding, BufferDescriptor, BufferUsages, Color, Device,
    PipelineLayout, ShaderModule, ShaderStages, SurfaceConfiguration, TextureDescriptor,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor, VertexBufferLayout,
};

use crate::{
    background::Background,
    camera::CameraUniform,
    fullscreen::FullscreenQuad,
    get_all, get_all_shaders,
    gpu::Gpu,
    idx_all, load_shader,
    shader_manager::{PipelineHandle, ShaderManager},
    ViewTextureReasources,
};

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

pub struct Bloom {
    down_chain: Chain,
    pub up_chain: Chain,
    extract_texture: wgpu::Texture,
    extract_texture_view: wgpu::TextureView,
    extract_bindings: wgpu::BindGroup,
}

impl BloomReasoruces {
    pub fn init(
        gpu: &Gpu,
        config: &SurfaceConfiguration,
        rr: &mut AnyMap,
        shader_manager: &mut ShaderManager,
    ) {
        let bind_group_layout = gpu
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Bloom bind group layout"),
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
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

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

        let sampler_bind_group_layout =
            gpu.device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("Bloom sampler bind group layout"),
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    }],
                });

        let texture_bind_group_layout =
            gpu.device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
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

        let fs_shader = rr.get::<FullscreenQuad>().unwrap().shader_handle;

        let down_pipeline = {
            let bloom_down_handle = load_shader!(shader_manager, "bloom_down.wgsl");
            struct DownPipelineLayout(PipelineLayout);
            rr.insert(DownPipelineLayout(gpu.device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Bloom down render pipeline layout"),
                    bind_group_layouts: &[&sampler_bind_group_layout, &texture_bind_group_layout],
                    push_constant_ranges: &[],
                },
            )));
            shader_manager.add_render_pipeline(
                vec![bloom_down_handle, fs_shader],
                Box::new(move |rr, device, shader_manager| {
                    let (fsq, dpl) = get_all!(rr, FullscreenQuad, DownPipelineLayout).unwrap();
                    let (bloom_shader, fs_shader) =
                        get_all_shaders!(shader_manager, bloom_down_handle, fs_shader).unwrap();

                    mk_pipeline(
                        "Bloom down render pipeline",
                        device,
                        &dpl.0,
                        fsq.vertex_format,
                        &fs_shader,
                        &bloom_shader,
                    )
                }),
            )
        };
        let up_pipeline = {
            let bloom_up_handle = load_shader!(shader_manager, "bloom_up.wgsl");
            struct UpPipelineLayout(PipelineLayout);
            rr.insert(UpPipelineLayout(gpu.device.create_pipeline_layout(
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
                vec![bloom_up_handle, fs_shader],
                Box::new(move |rr, device, shader_manager| {
                    let (fsq, upl) = get_all!(rr, FullscreenQuad, UpPipelineLayout).unwrap();
                    let (bloom_shader, fs_shader) =
                        get_all_shaders!(shader_manager, bloom_up_handle, fs_shader).unwrap();

                    mk_pipeline(
                        "Bloom up render pipeline",
                        device,
                        &upl.0,
                        fsq.vertex_format,
                        &fs_shader,
                        &bloom_shader,
                    )
                }),
            )
        };
        let extract_bind_group_layout = extract_bind_group_layout_name(gpu);
        let bloom_extract_handle = load_shader!(shader_manager, "bloom_extract.wgsl");
        struct ExtractPipelineLayout(PipelineLayout);
        rr.insert(ExtractPipelineLayout(gpu.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Bloom extract render pipeline layout"),
                bind_group_layouts: &[&extract_bind_group_layout],
                push_constant_ranges: &[],
            },
        )));

        let extract_pipeline = shader_manager.add_render_pipeline(
            vec![bloom_extract_handle, fs_shader],
            Box::new(move |rr, device, shader_manager| {
                let (fsq, epl) = get_all!(rr, FullscreenQuad, ExtractPipelineLayout).unwrap();
                let (bloom_shader, fs_shader) =
                    get_all_shaders!(shader_manager, bloom_extract_handle, fs_shader).unwrap();

                mk_pipeline(
                    "Bloom extract render pipeline",
                    device,
                    &epl.0,
                    fsq.vertex_format,
                    &fs_shader,
                    &bloom_shader,
                )
            }),
        );

        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
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

        let sampler_bind_group = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Bloom sampler bind group"),
            layout: &sampler_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Sampler(&sampler),
            }],
        });

        let threshold_buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("Threshold buffer"),
            size: std::mem::size_of::<f32>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bloom_reasources = Self {
            threshold_buffer,
            down_pipeline,
            up_pipeline,
            sampler,
            sampler_bind_group,
            texture_bind_group_layout,
            extract_pipeline,
            extract_bind_group_layout,
        };

        let threshold: f32 = 1.0;
        bloom_reasources.update_threshold(&gpu.queue, threshold);
        rr.insert(bloom_reasources);
    }

    fn update_threshold(&self, queue: &wgpu::Queue, threshold: f32) {
        queue.write_buffer(&self.threshold_buffer, 0, &threshold.to_ne_bytes());
    }
}

impl Bloom {
    pub fn get_result_bind_group(&self) -> &BindGroup {
        &self.up_chain.bind_groups.last().unwrap()
    }

    pub fn init(config: &SurfaceConfiguration, device: &Device, rr: &mut AnyMap) {
        let x = {
            let vt = &get_all!(rr, ViewTextureReasources)
                .unwrap()
                .0
                .view_texture
                .view;

            let tmp_tex = create_texture(device, (config.width, config.height));
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
            let mut x = Self {
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
            };

            x.resize(config, device, vt, rr);
            x
        };

        rr.insert(x);
    }

    pub fn resize(
        &mut self,
        config: &SurfaceConfiguration,
        device: &Device,
        view_texture_view: &TextureView,
        rr: &AnyMap,
    ) {
        self.down_chain.empty();
        self.up_chain.empty();

        // extract_texture: wgpu::Texture,
        self.extract_texture = create_texture(device, (config.width, config.height));
        self.extract_texture_view = self.extract_texture.create_view(&TextureViewDescriptor {
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
        self.extract_bindings = create_extract_bind_group(rr, device, view_texture_view);

        // down_chain: chain,
        {
            let (br,) = get_all!(rr, BloomReasoruces).unwrap();
            let lowest_x = 10;
            let mut width = config.width;
            let mut height = config.height;

            let tex = self
                .extract_texture
                .create_view(&TextureViewDescriptor::default());
            self.down_chain
                .bind_groups
                .push(mk_bind_group(device, br, &tex, width, height));

            while width > lowest_x {
                width /= 2;
                height /= 2;

                let tex = create_texture(device, (width, height));
                let view = tex.create_view(&TextureViewDescriptor::default());
                let bind_group = mk_bind_group(device, br, &view, width, height);

                self.down_chain.textures.push(tex);
                self.down_chain.texture_views.push(view);
                self.down_chain.bind_groups.push(bind_group);
            }

            for _ in 0..self.down_chain.bind_groups.len() {
                let tex = create_texture(device, (width, height));
                let view = tex.create_view(&TextureViewDescriptor::default());
                let bind_group = mk_bind_group(device, br, &view, width, height);

                self.up_chain.textures.push(tex);
                self.up_chain.texture_views.push(view);
                self.up_chain.bind_groups.push(bind_group);

                width *= 2;
                height *= 2;
            }

            println!(
                "Up: {}, Down {}",
                self.up_chain.textures.len(),
                self.down_chain.textures.len()
            )
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
    rr: &anymap::Map,
    device: &Device,
    view_texture_view: &TextureView,
) -> wgpu::BindGroup {
    let (br, bg, cam) = idx_all!(rr, BloomReasoruces, Background, CameraUniform);
    device.create_bind_group(&BindGroupDescriptor {
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
                resource: wgpu::BindingResource::TextureView(view_texture_view),
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
    })
}

fn extract_bind_group_layout_name(gpu: &Gpu) -> wgpu::BindGroupLayout {
    gpu.device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        },
        fragment: Some(wgpu::FragmentState {
            // 3.
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
        depth_stencil: None, // 1.
        multisample: wgpu::MultisampleState {
            count: 1,                         // 2.
            mask: !0,                         // 3.
            alpha_to_coverage_enabled: false, // 4.
        },
        multiview: None, // 5.
    })
}

pub fn draw_bloom(
    encoder: &mut wgpu::CommandEncoder,
    shader_manager: &ShaderManager,
    rr: &AnyMap,
) -> Option<()> {
    let (bloom, br, fullscreen_quad) = get_all!(rr, Bloom, BloomReasoruces, FullscreenQuad)?;

    if let Some(pipeline) = shader_manager.get_render_pipeline(&br.extract_pipeline) {
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
    }

    if let Some(pipeline) = shader_manager.get_render_pipeline(&br.down_pipeline) {
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
    }

    //up  :0 1 2 3 4 5
    //down:0 1 2 3 4 5

    if let Some(pipeline) = shader_manager.get_render_pipeline(&br.up_pipeline) {
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
    }
    Some(())
    // .for_each(|((view, self_texture), prev_texture)| {
    // });
}
