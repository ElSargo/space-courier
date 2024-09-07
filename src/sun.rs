use std::sync::{Arc, Mutex};

use anymap::AnyMap;
use bevy_ecs::{
    component::Component,
    query::With,
    system::{Query, Res, Resource},
};
use glam::Vec3;
use wgpu::{BindGroupLayout, Device};

use crate::{
    camera::{Camera, CameraUniform},
    ecs::Ecs,
    get_all,
    gpu::{Gpu, InstanceBuffer},
    instance_buffer_bytes, mk_pipeline, mk_pipeline_layout,
    model::Model,
    resources,
    shader_manager::{PipelineHandle, ShaderManager, ShaderStorage},
    transform::Transform,
    RawInstance,
};

pub(crate) const SUN_MASS: f32 = 20000.0;

pub fn update_instance_buffer(
    camera: Res<Camera>,
    transforms: Query<&Transform, With<Sun>>,
    ib: Res<SunIB>,
    gpu: Res<Gpu>,
    // ship_data: ResMut<ShipGpuResources>,
) {
    let ship_bytes = instance_buffer_bytes(transforms.iter(), camera.eye);
    ib.0.lock()
        .unwrap()
        .write(&gpu.device, &ship_bytes.as_slice(), &gpu.queue);
}

pub struct SunRenderReasources {
    pub sun_transform_instance_buffer: Arc<Mutex<InstanceBuffer>>,
    pub sun_render_pipeline: PipelineHandle,
    pub sun_model: Model,
}

#[derive(Resource)]
pub struct SunIB(Arc<Mutex<InstanceBuffer>>);

#[derive(Component)]
pub struct Sun;

pub async fn init(
    ecs: &mut Ecs,
    render_stuff: &mut AnyMap,
    shader_manager: &mut ShaderManager,
    gpu: &Gpu,
    texture_bind_group_layout: &BindGroupLayout,
) {
    let sun_model = resources::load_model(
        "wallmart.obj",
        &gpu.device,
        &gpu.queue,
        &texture_bind_group_layout,
    )
    .await
    .unwrap();

    // let sun_transforms = vec![];

    ecs.world.spawn((
        Sun,
        Transform {
            scale: Vec3::ONE * 20.0,
            ..Default::default()
        },
    ));

    // let sun_transform_buffer_data =
    //     instance_buffer_bytes_from_components(sun_transforms.iter().map(
    //         |Transform {
    //              scale,
    //              translation,
    //              rotation,
    //          }| (*translation, *rotation, *scale),
    //     ));
    let sun_transform_instance_buffer =
        Arc::new(Mutex::new(InstanceBuffer::new_init::<RawInstance>(
            Some("Sun instance Buffer".to_owned()),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            &gpu.device,
            // bytemuck::cast_slice(&sun_transform_buffer_data),
            &[],
        )));

    let layout = {
        let (cgs,) = get_all!(render_stuff, CameraUniform).unwrap();
        Box::leak(Box::new(mk_pipeline_layout(
            &gpu.device,
            "Sun",
            &[&cgs.inner.bind_group_layout, &texture_bind_group_layout],
        )))
    };

    ecs.world
        .insert_resource(SunIB(sun_transform_instance_buffer.clone()));
    render_stuff.insert({
        let sun_shader = shader_manager.load_shader("./src/sun.wgsl");

        SunRenderReasources {
            sun_transform_instance_buffer,
            sun_render_pipeline: shader_manager.add_render_pipeline(
                vec![sun_shader],
                Box::new(move |_rr: &AnyMap, device: &Device, sm: &ShaderStorage| {
                    // let (cgs,) = get_all!(rr, CameraUniform).unwrap();

                    mk_pipeline(
                        device,
                        &layout,
                        None,
                        &sm.get_module(sun_shader).unwrap(),
                        "sun",
                        RawInstance::desc(),
                    )
                }),
            ),
            sun_model,
        }
    });

    ecs.extract.add_systems(update_instance_buffer);
}
