use crate::shader_manager::SolidPipelineModuleResource;
use std::sync::Arc;

use bevy_ecs::{
    schedule::IntoSystemConfigs,
    system::{Commands, In, Query, Res, ResMut},
};
use wgpu::{
    Color, CommandEncoder, FragmentState, PipelineLayoutDescriptor, RenderPipeline, TextureFormat,
    TextureView,
};

use crate::{
    background::Background,
    camera::{CameraUniform, ReflectedCameraUniform},
    ecs::Initailiser,
    gpu::{InstanceBuffer, RenderPassExt, SolidDrawable, SolidRenderable, RD, RTBL, VTBGL},
    model::{self, Vertex},
    shader_manager::{
        Marker, PipelineHandle, PipelinesView, ShaderManagerCtrl,
        SolidPipelineModuleResourceExtras, SolidShader,
    },
    texture, RawInstance, ReflectedViewTextures, StandardRenderPipeline, ViewTextures,
};

pub struct Solidpass;

fn solid_pass(
    render_pipelines: Res<PipelinesView<RenderPipeline>>,
    mut cmd_encoder: ResMut<Marker<CommandEncoder, Solidpass>>,
    view_textures: Res<ViewTextures>,
    reflected_view_textures: Res<ReflectedViewTextures>,
    camera_uniform: Res<CameraUniform>,
    reflected_camera_uniform: Res<ReflectedCameraUniform>,
    background: Res<Background>,
    objects: Query<(&InstanceBuffer, &SolidRenderable)>,
) {
    for (camera_bind, color_view, depth_view, refl_bind) in [
        (
            &reflected_camera_uniform.bind_group,
            &reflected_view_textures.color_texture.view,
            &reflected_view_textures.depth_texture.view,
            &reflected_view_textures.null_bind_group,
        ),
        (
            &camera_uniform.bind_group,
            &view_textures.color_texture.view,
            &view_textures.depth_texture.view,
            &reflected_view_textures.bind_group,
        ),
    ] {
        let mut render_pass = pass(&mut cmd_encoder, color_view, depth_view);

        render_pass.set_bind_group(0, camera_bind, &[]);
        render_pass.set_bind_group(1, &background.texture_cube_bind_group, &[]);
        render_pass.set_bind_group(2, &refl_bind, &[]);

        let mut objects = objects
            .iter()
            .flat_map(|obj| {
                obj.1
                    .renderable
                    .aquire()
                    .map(|asset| (obj.1.pipeline_handle, obj.0, &obj.1.renderable, asset))
            })
            .collect::<Vec<_>>();
        objects.sort_by_key(|(pipeline_handle, ..)| *pipeline_handle);

        let mut last_pipeline = None;
        for (pipeline_handle, ib, renderable, asset) in objects.iter() {
            if Some(pipeline_handle) != last_pipeline {
                if let Some(pipeline) = render_pipelines.get(&pipeline_handle) {
                    render_pass.set_pipeline(&pipeline);
                    last_pipeline = Some(pipeline_handle);
                } else {
                    last_pipeline = None;
                    continue;
                }
            }

            render_pass.set_instance_buffer(1, ib);
            renderable.draw(&mut render_pass, 0..ib.len(), asset);
        }
    }
}

fn pass<'a>(
    cmd_encoder: &'a mut CommandEncoder,
    color_view: &TextureView,
    depth_view: &TextureView,
) -> wgpu::RenderPass<'a> {
    cmd_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Solid render pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &color_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(0.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        occlusion_query_set: None,
        timestamp_writes: None,
    })
}

pub fn init_mod(ecs: &mut Initailiser) {
    ecs.render.add_systems(solid_pass);
    ecs.init
        .add_systems(init_normal_pipeline.after(crate::gpu::mk_texture_bind_group_layout_resource));
}

pub fn init_normal_pipeline(shader_manager: Res<ShaderManagerCtrl>, mut commands: Commands) {
    let normal_shader = shader_manager.load_shader("./src/shader.wgsl");
    commands.insert_resource(StandardRenderPipeline {
        handle: shader_manager.add_render_pipeline(
            SolidShader {
                handle: normal_shader,
            },
            move |texture_bind_group_layout: RTBL| SolidPipelineModuleResourceExtras {
                vertex_buffers: Arc::new([model::ModelVertex::desc(), RawInstance::desc()]),
                extra_bind_groups: vec![texture_bind_group_layout.0.clone()],
            },
        ),
    })
}

pub fn solid_pass_render_descriptor<'a>(
    In(descriptor): In<SolidPipelineModuleResource>,
    device: RD,
    camera: Res<CameraUniform>,
    bg: Res<Background>,
    vtbgl: VTBGL,
) -> wgpu::RenderPipeline {
    let a = [
        &camera.bind_group_layout,
        &bg.texture_cube_bind_group_layout,
        &vtbgl,
    ];

    let b = descriptor
        .extras
        .extra_bind_groups
        .iter()
        .map(|x| x.as_ref())
        .collect::<Vec<_>>();

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("tri mesh pipeline layout"),
        bind_group_layouts: [a.as_slice(), b.as_slice()]
            .iter()
            .flat_map(|x| x.iter().map(|x| *x))
            .collect::<Vec<_>>()
            .as_slice(),
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Tri mesh render pipeline"),
        layout: Some(&pipeline_layout),
        cache: None,
        vertex: wgpu::VertexState {
            module: &descriptor.vertex,
            entry_point: "vs_main",
            buffers: &descriptor.extras.vertex_buffers,
            compilation_options: Default::default(),
        },
        fragment: Some(FragmentState {
            module: &descriptor.fragment,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: TextureFormat::Rgba16Float,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: texture::Texture::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Greater, // 1.
            stencil: wgpu::StencilState::default(),        // 2.
            bias: wgpu::DepthBiasState::default(),
        }), // 1.
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
        multisample: wgpu::MultisampleState {
            count: 1,                         // 2.
            mask: !0,                         // 3.
            alpha_to_coverage_enabled: false, // 4.
        },
        multiview: None, // 5.
    })
}
