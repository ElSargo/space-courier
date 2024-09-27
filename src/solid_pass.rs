use std::sync::Arc;

use bevy_ecs::{
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, ResMut},
};
use wgpu::{Color, CommandEncoder, RenderPipeline};

use crate::{
    background::Background,
    camera::CameraUniform,
    ecs::Initailiser,
    gpu::{InstanceBuffer, RenderPassExt, SolidDrawable, SolidRenderable, RTBL},
    model::{self, Vertex},
    shader_manager::{
        Marker, PipelineHandle, PipelinesView, ShaderManagerCtrl,
        SolidPipelineModuleResourceExtras, SolidShader,
    },
    RawInstance, StandardRenderPipeline, ViewTextureReasources,
};

pub struct Solidpass;

fn solid_pass(
    render_pipelines: Res<PipelinesView<RenderPipeline>>,
    mut cmd_encoder: ResMut<Marker<CommandEncoder, Solidpass>>,
    view_textures: Res<ViewTextureReasources>,
    camera_uniform: Res<CameraUniform>,
    background: Res<Background>,
    objects: Query<(&InstanceBuffer, &SolidRenderable)>,
) {
    let mut render_pass = cmd_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Solid render pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view_textures.color_texture.view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &view_textures.depth_texture.view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(0.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        occlusion_query_set: None,
        timestamp_writes: None,
    });

    render_pass.set_bind_group(0, &camera_uniform.bind_group, &[]);
    render_pass.set_bind_group(1, &background.texture_cube_bind_group, &[]);

    let mut objects: Vec<(&InstanceBuffer, PipelineHandle, Arc<dyn SolidDrawable>)> = objects
        .iter()
        .flat_map(|obj| {
            obj.1
                .renderable
                .aquire()
                .map(|renderable| (obj.0, obj.1.pipeline_handle, renderable))
        })
        .collect::<Vec<_>>();
    objects.sort_by_key(|(_, pipeline_handle, _)| *pipeline_handle);

    let mut last_pipeline = None;
    for (ib, pipeline_handle, renderable) in objects.iter() {
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
        renderable.draw(&mut render_pass, 0..ib.len());
    }
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
