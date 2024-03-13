use crate::{
    fullscreen::FullscreenQuad,
    shader_manager::{self, PipelineHandle},
};
use wgpu::{BindGroupLayoutDescriptor, BindGroupLayoutEntry, Device, ShaderStages};

pub struct Postprocessing {
    pipeline_handle: PipelineHandle,
}

impl Postprocessing {
    pub fn new(
        device: &Device,
        config: &wgpu::SurfaceConfiguration,
        fullscreen_quad: &FullscreenQuad,
        view_textures_bind_group_layout: &wgpu::BindGroupLayout,
        shader_manager: &mut shader_manager::ShaderManager,
    ) -> Self {
        let shader_handle = shader_manager.load_shader("./src/post.wgsl");

        // let post_shader = device.create_shader_module(wgpu::include_wgsl!());

        let inverse_camera = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("inverse_camera_bind_group_layout"),
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

        let pipeline_layout = Box::leak(Box::new(device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("post pipline layout"),
                bind_group_layouts: &[
                    &view_textures_bind_group_layout,
                    &inverse_camera, // &model_transform_bind_group_layout,
                    &texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            },
        )));
        // let pipeline = device.create_render_pipeline(&);
        let vert_fmt = fullscreen_quad.vertex_format;
        let format = config.format.clone();
        let pipeline_handle = shader_manager.add_pipeline(
            vec![shader_handle, fullscreen_quad.shader_handle],
            Box::new(move |shader_modules, device: &Device| {
                let targets = [Some(wgpu::ColorTargetState {
                    // 4.
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })];
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Post Render Pipeline"),
                    layout: Some(pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: shader_modules[1],
                        entry_point: "vs_main",
                        buffers: vert_fmt,
                    },
                    fragment: Some(wgpu::FragmentState {
                        // 3.
                        module: shader_modules[0],
                        entry_point: "fs_main",
                        targets: &targets,
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
            }),
        );

        Self { pipeline_handle }
    }

    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: wgpu::TextureView,
        fullscreen_quad: &FullscreenQuad,
        camera_bind_group: &wgpu::BindGroup,
        textures_bind_group: &wgpu::BindGroup,
        bloom_texture_bind_group: &wgpu::BindGroup,
        shader_manager: &shader_manager::ShaderManager,
    ) {
        if let Some(pipeline) = shader_manager.get_pipeline(&self.pipeline_handle) {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Ui render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            // render_pass.set_bind_group(0,&self.diffuse_bind_group , &[] );
            // render_pass.set_bind_group(0,&self.view_bind_group, &[] );
            render_pass.set_bind_group(0, &textures_bind_group, &[]);
            render_pass.set_bind_group(1, camera_bind_group, &[]);
            render_pass.set_bind_group(2, bloom_texture_bind_group, &[]);
            render_pass.set_pipeline(pipeline);
            fullscreen_quad.draw(&mut render_pass);
        }
    }
}
