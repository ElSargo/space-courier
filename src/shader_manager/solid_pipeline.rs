use std::{marker::PhantomData, sync::Arc};

use crate::{shader_manager::ShaderHandle, solid_pass::solid_pass_render_descriptor};
use bevy_ecs::{
    system::{IntoSystem, SystemId},
    world::World,
};
use wgpu::{BindGroupLayout, ShaderModule, VertexBufferLayout};

use super::{
    pipeline_shader_resource::PipelineShaderDescriptor, PipelineConstructor,
    PipelineModuleDescriptor,
};

pub struct SolidSplitShaders {
    pub vertex: ShaderHandle,
    pub fragment: ShaderHandle,
}

pub struct SolidShader {
    pub handle: ShaderHandle,
}

impl<M, S: IntoSystem<(), SolidPipelineModuleResourceExtras, M> + 'static>
    PipelineShaderDescriptor<1, wgpu::RenderPipeline, S, M> for SolidShader
{
    type Descriptor = SolidPipelineModuleDescriptor<1, S, M>;

    fn into_descriptor(self, system: S) -> SolidPipelineModuleDescriptor<1, S, M> {
        SolidPipelineModuleDescriptor {
            shaders: [self.handle],
            system,
            phantom: PhantomData,
        }
    }
}

impl<M, S: IntoSystem<(), SolidPipelineModuleResourceExtras, M> + 'static>
    PipelineShaderDescriptor<2, wgpu::RenderPipeline, S, M> for SolidSplitShaders
{
    type Descriptor = SolidPipelineModuleDescriptor<2, S, M>;

    fn into_descriptor(self, system: S) -> SolidPipelineModuleDescriptor<2, S, M> {
        SolidPipelineModuleDescriptor {
            shaders: [self.vertex, self.fragment],
            system,
            phantom: PhantomData,
        }
    }
}

pub struct SolidPipelineModuleDescriptor<const N: usize, S, M>
where
    S: IntoSystem<(), SolidPipelineModuleResourceExtras, M> + 'static,
{
    shaders: [ShaderHandle; N],
    system: S,
    phantom: PhantomData<M>,
}

pub struct SolidPipelineModuleResourceExtras {
    pub vertex_buffers: Arc<[VertexBufferLayout<'static>]>,
    pub extra_bind_groups: Vec<Arc<BindGroupLayout>>,
    pub label: Option<&'static str>,
}

pub struct SolidPipelineModuleResource {
    pub vertex: Arc<ShaderModule>,
    pub fragment: Arc<ShaderModule>,
    pub extras: SolidPipelineModuleResourceExtras,
}

impl<
        'a,
        const N: usize,
        System: bevy_ecs::system::IntoSystem<(), SolidPipelineModuleResourceExtras, M>,
        M,
    > PipelineModuleDescriptor<N, wgpu::RenderPipeline, System, M>
    for SolidPipelineModuleDescriptor<N, System, M>
{
    // type Shaders = SolidSplitShaders;
    fn constructor(self, world: &mut World) -> SolidPipelineConstructor {
        SolidPipelineConstructor {
            descriptor_system_id: world.register_system(self.system),
            system_id: world.register_system(solid_pass_render_descriptor),
        }
    }

    fn as_arr(&self) -> [ShaderHandle; N] {
        self.shaders
    }
}

impl PipelineConstructor<wgpu::RenderPipeline> for SolidPipelineConstructor {
    fn construct(&mut self, moduels: &[Arc<ShaderModule>], rr: &mut World) -> wgpu::RenderPipeline {
        let extras = rr.run_system(self.descriptor_system_id).unwrap();
        rr.run_system_with_input(
            self.system_id,
            SolidPipelineModuleResource {
                extras,
                vertex: moduels[0].clone(),
                fragment: moduels.get(1).unwrap_or(&moduels[0]).clone(),
            },
        )
        .unwrap()
    }
}

pub struct SolidPipelineConstructor {
    descriptor_system_id: SystemId<(), SolidPipelineModuleResourceExtras>,
    system_id: SystemId<SolidPipelineModuleResource, wgpu::RenderPipeline>,
}
