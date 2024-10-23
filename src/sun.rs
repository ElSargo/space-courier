use crate::{prelude::*, standard_object};

pub(crate) const SUN_MASS: f32 = 10.0;

#[derive(Component)]
pub struct Sun;

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

pub fn init_mod(ecs: &mut Initailiser) {
    ecs.init.add_systems((spawn_sun,));

    standard_object!(
        ecs,
        "wallmart.obj",
        Sun,
        With<Sun>,
        With<Sun>,
        (|shader_manager: Res<ShaderLoader>| {
            shader_manager.add_render_pipeline(
                SolidShader {
                    handle: shader_manager.load_shader("./src/sun.wgsl"),
                },
                move || SolidPipelineModuleResourceExtras {
                    vertex_buffers: [model::ModelVertex::desc(), RawInstance::desc()].into(),
                    label: Some("Sun pipeline"),
                    ..Default::default()
                },
            )
        })
    )
}

/* test.toml




*/
