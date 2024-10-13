use crate::shader_manager::SolidShader;
use crate::{
    gpu::{self, pack_buffers, RD, RQ, RTBL},
    marker::Marker,
    model, model_loader,
    physics::Physics,
    renderables::SolidRenderable,
    shader_manager::SolidPipelineModuleResourceExtras,
};
use bevy_ecs::prelude::Res;
use bevy_ecs::schedule::IntoSystemConfigs;
use bevy_ecs::system::Commands;

use glam::vec3;
use glam::Quat;

use bevy_ecs::{component::Component, query::With, system::Resource};
use glam::Vec3;

use crate::{
    camera::CameraUniform, ecs::Initailiser, instance_buffer::InstanceBuffer, mk_pipeline_layout,
    shader_manager::ShaderLoader, transform::Transform, RawInstance,
};

pub(crate) const SUN_MASS: f32 = 20000.0;

#[derive(Component)]
pub struct Sun;

pub fn init(
    device: RD,
    queue: RQ,
    shader_manager: Res<ShaderLoader>,
    texture_bind_group_layout: RTBL,
    cgs: Res<CameraUniform>,
    mut commands: Commands,
) {
    // device: Res<DeviceRes>,
    {
        let sun_model = model_loader::load_model(
            "wallmart.obj",
            device.0.clone(),
            queue.0.clone(),
            texture_bind_group_layout.0.clone(),
        );

        let sun_transform_instance_buffer = InstanceBuffer::new_init::<RawInstance>(
            Some("Sun instance Buffer".to_owned()),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            &device,
            // bytemuck::cast_slice(&sun_transform_buffer_data),
            &[],
        );

        commands.insert_resource::<Marker<wgpu::PipelineLayout, SunPipelineLayout>>(Marker::new(
            mk_pipeline_layout(
                &device,
                "Sun",
                &[&cgs.bind_group_layout, &texture_bind_group_layout.0],
            ),
        ));

        let sun_shader = shader_manager.load_shader("./src/sun.wgsl");
        commands.spawn((
            SolidRenderable {
                pipeline: shader_manager.add_render_pipeline(
                    SolidShader { handle: sun_shader },
                    move || SolidPipelineModuleResourceExtras {
                        vertex_buffers: [model::ModelVertex::desc(), RawInstance::desc()].into(),
                        label: Some("Sun pipeline"),
                        ..Default::default()
                    },
                ),
                renderable: Box::new(sun_model),
            },
            Sun,
            sun_transform_instance_buffer,
        ));
    }
}

fn spawn_sun(mut commands: Commands) {
    commands.spawn((
        Transform {
            translation: vec3(50.0, 10.0, 50.0),
            ..Default::default()
        },
        Physics {
            velocity: Vec3::ZERO,
            angular_velocity: Quat::IDENTITY,
            mass: 10e10,
        },
        Sun,
    ));
}

#[derive(Resource)]
struct SunPipelineLayout;

pub fn init_mod(ecs: &mut Initailiser) {
    // let pipeline_constructor = ecs.world.register_system(pipeline_constructor);

    ecs.init.add_systems((
        spawn_sun,
        init.after(gpu::mk_texture_bind_group_layout_resource),
    ));

    ecs.prepare_buffers
        .add_systems(pack_buffers::<RawInstance, With<Sun>, With<Sun>>);
}
