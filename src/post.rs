use crate::{
    gpu::{OutputTextureResource, ScreenFormat, VTBGL},
    planet_shadows::PlanetShadows,
    prelude::*,
    Background, ViewTextures,
};

#[derive(Resource)]
pub struct Postprocessing {
    pipeline_handle: PipelineHandle,
}

impl Postprocessing {
    pub fn new(
        device: RD,
        fullscreen_quad: Res<FullscreenQuad>,
        planet_shadows: Option<Res<PlanetShadows>>,
        bg: Res<Background>,
        shader_manager: Res<ShaderLoader>,
        mut commands: Commands,
        vtbgl: VTBGL,
    ) {
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
                label: Some("Texture bind group layout"),
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

        let pipeline_layout = Box::leak(Box::new(
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("post pipline layout"),
                bind_group_layouts: [
                    Some(&vtbgl.0),
                    Some(&inverse_camera), // &model_transform_bind_group_layout,
                    Some(&texture_bind_group_layout),
                    Some(&bg.texture_cube_bind_group_layout),
                    planet_shadows
                        .as_ref()
                        .map(|pl| &pl.texture_bind_group_layout),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .as_slice(),
                push_constant_ranges: &[],
            }),
        ));
        // let pipeline = device.create_render_pipeline(&);
        // let format = Box::leak(Box::new(resolution.format.clone()));
        let pipeline_handle = shader_manager.add_render_pipeline(
            // vec![shader_handle, fullscreen_quad.shader_handle],
            SplitShaders {
                vertex: fullscreen_quad.shader_handle,
                fragment: shader_handle,
            },
            // Box::new(move |rr, device, modules| {
            |In(SplitPipelineModuleResource { vertex, fragment }),
             fsq: Res<FullscreenQuad>,
             format: Res<ScreenFormat>,
             device: RD| {
                let targets = [Some(wgpu::ColorTargetState {
                    // 4.
                    format: **format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })];
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Post Render Pipeline"),
                    layout: Some(pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &vertex,
                        entry_point: "vs_main",
                        buffers: fsq.vertex_format,
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        // 3.
                        module: &fragment,
                        entry_point: "fs_main",
                        targets: &targets,
                        compilation_options: Default::default(),
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
            },
        );

        commands.insert_resource(Self { pipeline_handle });
    }
}

pub fn draw_post(
    mut encoder: ResMut<Marker<CommandEncoder, Postprocessing>>,
    output_view: Res<OutputTextureResource>,
    render_pipelines: Res<PipelinesView<RenderPipeline>>,
    vtr: Res<ViewTextures>,
    post: Res<Postprocessing>,
    fsq: Res<FullscreenQuad>,
    camera_uniform: Res<CameraUniform>,
    br: Res<Bloom>,
    pl: Option<Res<PlanetShadows>>,
    bg: Res<Background>,
    mut flip: Local<bool>,
    kb: Res<ButtonInput>,
) {
    if let Some(pipeline) = render_pipelines.get(post.pipeline_handle) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Post render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &output_view,
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
        if kb.just_down(KeyCode::KeyF) {
            *flip = !*flip;
        }
        render_pass.set_bind_group(0, &vtr.bind_group, &[]);
        render_pass.set_bind_group(1, &camera_uniform.bind_group, &[]);
        render_pass.set_bind_group(2, &br.get_result_bind_group(), &[]);
        render_pass.set_bind_group(3, &bg.texture_cube_bind_group, &[]);
        if let Some(pl) = pl {
            render_pass.set_bind_group(4, &pl.render_texture_bind_group, &[]);
        }
        render_pass.set_pipeline(&pipeline);
        // println!("Post drawn");
        fsq.draw(&mut render_pass);
    } else {
        println!("No post pipeline");
    }
}

pub fn init_mod(initialiser: &mut Initailiser) {
    initialiser.init.add_systems(
        Postprocessing::new
            .after(crate::fullscreen::init_fsq)
            .after(crate::bloom::init_res)
            .after(crate::background::init_background),
    );
    initialiser.render.add_systems(draw_post);
}
