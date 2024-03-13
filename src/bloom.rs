use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BufferDescriptor, BufferUsages, Color, Device, ShaderModule, ShaderStages,
    SurfaceConfiguration, TextureDescriptor, TextureFormat, TextureView, VertexBufferLayout,
};

use crate::{
    fullscreen::FullscreenQuad,
    shader_manager::{PipelineHandle, ShaderManager},
};

pub struct Bloom {
    down_textures: Vec<wgpu::Texture>,
    down_texture_views: Vec<wgpu::TextureView>,
    pub down_texture_bind_groups: Vec<wgpu::BindGroup>,
    up_textures: Vec<wgpu::Texture>,
    up_texture_views: Vec<wgpu::TextureView>,
    pub up_texture_bind_groups: Vec<wgpu::BindGroup>,
    sampler_bind_group: wgpu::BindGroup,
    up_pipeline: PipelineHandle,
    down_pipeline: PipelineHandle,
    threshold_bind_group: wgpu::BindGroup,
    threshold_buffer: wgpu::Buffer,
    threshold_nill_bind_group: wgpu::BindGroup,
    threshold_buffer_nill: wgpu::Buffer,
    view_texture_bind_group: Option<wgpu::BindGroup>,
}

impl Bloom {
    pub fn new(
        device: &Device,
        config: &SurfaceConfiguration,
        fullscreen_quad: &FullscreenQuad,
        view_textures_view: &TextureView,
        shader_manager: &mut ShaderManager,
    ) -> Self {
        let threshold_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Bloom bind group lauout"),
            });

        let sampler_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Bloom sampler bind group layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                }],
            });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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

        let pipeline_layout: &'static mut wgpu::PipelineLayout =
            Box::<wgpu::PipelineLayout>::leak(Box::new(device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("bloom pipline layout"),
                    bind_group_layouts: &[
                        &sampler_bind_group_layout,
                        &threshold_bind_group_layout,
                        &texture_bind_group_layout,
                        &texture_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                },
            )));

        let bloom_shader_handle = shader_manager.load_shader("./src/bloom.wgsl");

        let down_pipeline = shader_manager.add_pipeline(
            vec![bloom_shader_handle, fullscreen_quad.shader_handle],
            Box::new(|shader_moules, device| {
                mk_layout(
                    device,
                    pipeline_layout,
                    fullscreen_quad.vertex_format,
                    "down",
                    shader_moules[1],
                    shader_moules[0],
                )
            }),
        );
        let up_pipeline = shader_manager.add_pipeline(
            vec![bloom_shader_handle, fullscreen_quad.shader_handle],
            Box::new(|shader_moules, device| {
                mk_layout(
                    device,
                    pipeline_layout,
                    fullscreen_quad.vertex_format,
                    "up",
                    shader_moules[1],
                    shader_moules[0],
                )
            }),
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

        let threshold_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Threshold buffer"),
            size: std::mem::size_of::<f32>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let threshold_buffer_nill = device.create_buffer(&BufferDescriptor {
            label: Some("Threshold nill buffer"),
            size: std::mem::size_of::<f32>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let threshold_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Bloom threshold bind group"),
            layout: &threshold_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &threshold_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        let threshold_nill_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Bloom threshold nill bind group"),
            layout: &threshold_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &threshold_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let sampler_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Bloom sampler bind group"),
            layout: &sampler_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Sampler(&sampler),
            }],
        });

        let mut un_init = Self {
            down_textures: vec![],
            down_texture_views: vec![],
            down_texture_bind_groups: vec![],
            up_textures: vec![],
            up_texture_views: vec![],
            up_texture_bind_groups: vec![],
            up_pipeline,
            down_pipeline,
            threshold_bind_group,
            threshold_buffer,
            sampler_bind_group,
            view_texture_bind_group: None,
            threshold_nill_bind_group,
            threshold_buffer_nill,
        };

        un_init.resize(config, device, view_textures_view);

        un_init
    }

    pub fn resize(
        &mut self,
        config: &SurfaceConfiguration,
        device: &Device,
        view_texture_view: &TextureView,
    ) {
        self.down_textures.clear();
        self.down_texture_views.clear();
        self.down_texture_bind_groups.clear();
        self.up_textures.clear();
        self.up_texture_views.clear();
        self.up_texture_bind_groups.clear();

        let lowest_x = 40;
        let mut width = config.width / 2;
        let mut height = config.height / 2;

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Bloom bind group layout"),
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

        self.view_texture_bind_group = Some(device.create_bind_group(&BindGroupDescriptor {
            label: Some("Bloom view texture bind group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view_texture_view),
            }],
        }));

        while lowest_x < width {
            let down_texture = device.create_texture(&TextureDescriptor {
                label: Some(&format!("Bloom down buffer {width} {height}")),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT // 3.
        | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            let view = down_texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some(&format!("Bloom down view {width} {height}")),
                format: Some(TextureFormat::Rgba16Float),
                ..Default::default()
            });

            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some(&format!("Bloom down {width} {height} bind group")),
                layout: &bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                }],
            });
            self.down_textures.push(down_texture);
            self.down_texture_views.push(view);
            self.down_texture_bind_groups.push(bind_group);

            let up_texture = device.create_texture(&TextureDescriptor {
                label: Some(&format!("Bloom up buffer {width} {height}")),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT // 3.
        | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            let view = up_texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some(&format!("Bloom up view {width} {height}")),
                format: Some(TextureFormat::Rgba16Float),
                ..Default::default()
            });

            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some(&format!("Bloom up {width} {height} bind group")),
                layout: &bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                }],
            });
            self.up_textures.push(up_texture);
            self.up_texture_views.push(view);
            self.up_texture_bind_groups.push(bind_group);
            width /= 2;
            height /= 2;
        }
    }

    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        fullscreen_quad: &FullscreenQuad,
        shader_manager: &ShaderManager,
    ) {
        let threshold: f32 = 2.0;
        self.update_threshold(queue, threshold);
        if let Some(pipeline) = shader_manager.get_pipeline(&self.down_pipeline) {
            for i in 0..self.down_textures.len() {
                // println!("Down: {:#?}", self.down_textures[i].size());
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Bloom down render pass {i}"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.down_texture_views[i],
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
                render_pass.set_bind_group(0, &self.sampler_bind_group, &[]);
                if i == 0 {
                    render_pass.set_bind_group(1, &self.threshold_bind_group, &[]);
                } else {
                    render_pass.set_bind_group(1, &self.threshold_nill_bind_group, &[]);
                }

                if let Some(index) = i.checked_sub(1) {
                    render_pass.set_bind_group(2, &self.down_texture_bind_groups[index], &[]);
                } else {
                    render_pass.set_bind_group(
                        2,
                        self.view_texture_bind_group
                            .as_ref()
                            .expect("No view texture bind group set"),
                        &[],
                    );
                }

                // Dummy
                render_pass.set_bind_group(3, &self.up_texture_bind_groups[0], &[]);
                render_pass.set_pipeline(&pipeline);
                fullscreen_quad.draw(&mut render_pass);
            }
        }

        //up  :0 1 2 3 4 5
        //down:0 1 2 3 4 5

        if let Some(pipeline) = shader_manager.get_pipeline(&self.up_pipeline) {
            let mut i = self.down_textures.len() - 1;
            while i > 0 {
                let target_index = i - 1;
                let view = &self.up_texture_views[target_index];
                let self_texture = &self.down_texture_bind_groups[target_index];
                let prev_texture = &self.up_texture_bind_groups[i];
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

                render_pass.set_bind_group(0, &self.sampler_bind_group, &[]);
                render_pass.set_bind_group(1, &self.threshold_nill_bind_group, &[]);
                render_pass.set_bind_group(2, &self_texture, &[]);
                render_pass.set_bind_group(3, &prev_texture, &[]);
                render_pass.set_pipeline(&pipeline);
                fullscreen_quad.draw(&mut render_pass);
                i -= 1;
            }
        }
        // .for_each(|((view, self_texture), prev_texture)| {
        // });
    }

    fn update_threshold(&self, queue: &wgpu::Queue, threshold: f32) {
        queue.write_buffer(&self.threshold_buffer, 0, &threshold.to_ne_bytes());
        queue.write_buffer(&self.threshold_buffer_nill, 0, &0.0_f32.to_ne_bytes());
    }
}

fn mk_layout(
    device: &Device,
    pipeline_layout: &wgpu::PipelineLayout,
    vertex_layout: &'static [VertexBufferLayout<'static>],
    entry_point: &str,
    fullscreen_shader: &ShaderModule,
    bloom_shader: &ShaderModule,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Bloom render pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: fullscreen_shader,
            entry_point: "vs_main",
            buffers: vertex_layout,
        },
        fragment: Some(wgpu::FragmentState {
            // 3.
            module: &bloom_shader,
            entry_point,
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
