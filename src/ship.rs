use bevy_ecs::{
    component::Component,
    query::With,
    schedule::IntoSystemConfigs,
    system::{Commands, Res},
};

use crate::{
    background,
    ecs::Initailiser,
    gpu::{pack_buffers, TextureBindGroupLayoutResource, RD, RQ},
    instance_buffer::InstanceBuffer,
    model_loader,
    renderables::SolidRenderable,
    RawInstance, StandardRenderPipeline,
};

#[derive(Component)]
pub struct Ship;

pub fn init_ship(
    device: RD,
    mut commands: Commands,
    tr: Res<TextureBindGroupLayoutResource>,
    pipeline: Res<StandardRenderPipeline>,
    queue: RQ,
) {
    let ship_model = model_loader::load_model(
        "retroShip.obj",
        device.0.clone(),
        queue.0.clone(),
        tr.0.clone(),
    );

    commands.spawn((
        SolidRenderable {
            renderable: Box::new(ship_model),
            pipeline: pipeline.handle,
        },
        Ship,
        InstanceBuffer::new(
            Some("Ship instance Buffer".to_owned()),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            &device,
        ),
    ));
}

pub fn init_mod(ecs: &mut Initailiser) {
    ecs.prepare_buffers
        .add_systems(pack_buffers::<RawInstance, With<Ship>, With<Ship>>);
    // ecs.prepare_buffers.add_systems(update_instance_buffer);
    ecs.init.add_systems(
        init_ship
            .after(background::init_background)
            .after(crate::solid_pass::init_normal_pipeline),
    );
}
