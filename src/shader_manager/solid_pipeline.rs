use std::sync::Arc;

use crate::{shader_manager::ShaderHandle, solid_pass::solid_pass_render_descriptor};
use bevy_ecs::{system::SystemId, world::World};
use wgpu::{BindGroupLayout, PushConstantRange, ShaderModule, VertexBufferLayout};

use super::{pipeline_shader_resource::ShadersD, PipelineConstructor};

pub struct SolidShaders {
    pub vertex: ShaderHandle,
    pub fragment: ShaderHandle,
}

pub struct SolidShader {
    pub handle: ShaderHandle,
}

#[derive(Default)]
pub struct SolidPipelineModuleResourceExtras {
    pub vertex_buffers: Arc<[VertexBufferLayout<'static>]>,
    pub extra_bind_groups: Vec<Arc<BindGroupLayout>>,
    pub push_constants: Vec<PushConstantRange>,
    pub label: Option<&'static str>,
}

pub struct SolidPipelineModuleResource {
    pub vertex: Arc<ShaderModule>,
    pub fragment: Arc<ShaderModule>,
    pub extras: SolidPipelineModuleResourceExtras,
}

impl<System, M> ShadersD<wgpu::RenderPipeline, System, (), SolidPipelineModuleResourceExtras, M>
    for SolidShader
where
    System: bevy_ecs::system::IntoSystem<(), SolidPipelineModuleResourceExtras, M> + 'static,
{
    fn handles(&self) -> Box<[ShaderHandle]> {
        Box::new([self.handle])
    }

    fn constructor(
        self,
        system: System,
        world: &mut World,
    ) -> impl PipelineConstructor<wgpu::RenderPipeline> + 'static {
        let extras_system = world.register_system(system);
        let constructor_system = world.register_system(solid_pass_render_descriptor);

        construct(extras_system, constructor_system)
    }
}

impl<System, M> ShadersD<wgpu::RenderPipeline, System, (), SolidPipelineModuleResourceExtras, M>
    for SolidShaders
where
    System: bevy_ecs::system::IntoSystem<(), SolidPipelineModuleResourceExtras, M> + 'static,
{
    fn handles(&self) -> Box<[ShaderHandle]> {
        Box::new([self.vertex, self.fragment])
    }

    fn constructor(
        self,
        system: System,
        world: &mut World,
    ) -> impl PipelineConstructor<wgpu::RenderPipeline> + 'static {
        let extras_system = world.register_system(system);
        let constructor_system = world.register_system(solid_pass_render_descriptor);

        construct(extras_system, constructor_system)
    }
}

fn construct(
    extras_system: SystemId<(), SolidPipelineModuleResourceExtras>,
    constructor_system: SystemId<SolidPipelineModuleResource, wgpu::RenderPipeline>,
) -> impl Fn(&[Arc<ShaderModule>], &mut World) -> wgpu::RenderPipeline {
    move |moduels: &[Arc<ShaderModule>], rr: &mut World| {
        let extras = rr.run_system(extras_system).unwrap();
        rr.run_system_with_input(
            constructor_system,
            SolidPipelineModuleResource {
                extras,
                vertex: moduels[0].clone(),
                fragment: moduels.get(1).unwrap_or(&moduels[0]).clone(),
            },
        )
        .unwrap()
    }
}
