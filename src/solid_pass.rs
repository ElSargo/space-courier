use bevy_ecs::{
    schedule::IntoSystemConfigs,
    system::{Commands, In, Query, Res, ResMut, Resource},
};
use wgpu::{Color, CommandEncoder, PipelineLayout, RenderPipeline};

use crate::{
    background::Background,
    camera::CameraUniform,
    ecs::Initailiser,
    gpu::{InstanceBuffer, RenderPassExt, SolidRenderable, RD, RTBL},
    mk_pipeline, mk_pipeline_layout,
    model::DrawModel,
    shader_manager::{
        Marker, PipelinesView, ShaderManagerCtrl, UnifiedPipelineModuleDescriptor,
        UnifiedPipelineModuleResource,
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

    render_pass.set_bind_group(0, &camera_uniform.inner.bind_group, &[]);
    render_pass.set_bind_group(1, &background.texture_cube_bind_group, &[]);

    let mut objects = objects
        .iter()
        .flat_map(|obj| obj.1.model.get().map(|mo| (obj.0, obj.1, mo)))
        .collect::<Vec<_>>();
    objects.sort_by_key(|(_, sr, _)| sr.pipeline_handle);

    // println!("Rendering solid pass");
    // println!("Objects: {objects:#?}");

    let mut last_pipeline = None;
    for (ib, sr, mo) in objects.iter() {
        if Some(sr.pipeline_handle) != last_pipeline {
            if let Some(pipeline) = render_pipelines.get(&sr.pipeline_handle) {
                render_pass.set_pipeline(&pipeline);
                last_pipeline = Some(sr.pipeline_handle);
            } else {
                last_pipeline = None;
                continue;
            }
        }

        render_pass.set_instance_buffer(1, ib);
        render_pass.draw_model_instanced(mo.as_ref(), 0..ib.len());
    }
}

pub fn init_mod(ecs: &mut Initailiser) {
    ecs.render.add_systems(solid_pass);
    ecs.init
        .add_systems(init_normal_pipeline.after(crate::gpu::mk_texture_bind_group_layout_resource));
}

pub fn init_normal_pipeline(
    shader_manager: Res<ShaderManagerCtrl>,
    texture_bind_group_layout: RTBL,
    device: RD,
    background: Res<Background>,
    mut commands: Commands, // let (gpu, background) = get_all!(render_stuff, Gpu, Background).unwrap();
    cam_uniform: Res<CameraUniform>,
) {
    #[derive(Resource)]
    struct NormalPiplineLayout {
        layout: PipelineLayout,
    }
    let normal_shader = shader_manager.load_shader("./src/shader.wgsl");

    commands.insert_resource(NormalPiplineLayout {
        layout: mk_pipeline_layout(
            &device.clone(),
            "normal",
            &[
                &cam_uniform.inner.bind_group_layout,
                &background.texture_cube_bind_group_layout,
                &texture_bind_group_layout,
            ],
        ),
    });
    commands.insert_resource(StandardRenderPipeline {
        handle: shader_manager.add_render_pipeline(
            UnifiedPipelineModuleDescriptor {
                handle: normal_shader,
            },
            move |In(module): In<UnifiedPipelineModuleResource>,
                  layout: Res<NormalPiplineLayout>,
                  device: RD| {
                mk_pipeline(
                    &device,
                    &layout.layout,
                    None,
                    &module,
                    "normal",
                    RawInstance::desc(),
                )
            },
        ),
    });
}
