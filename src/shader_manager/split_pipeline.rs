use std::{marker::PhantomData, sync::Arc};

use crate::shader_manager::ShaderHandle;
use bevy_ecs::{
    system::{IntoSystem, SystemId},
    world::World,
};
use wgpu::ShaderModule;

use super::{
    pipeline_shader_resource::PipelineShaderDescriptor, PipelineConstructor,
    PipelineModuleDescriptor,
};

pub struct SplitShaders {
    pub vertex: ShaderHandle,
    pub fragment: ShaderHandle,
}

impl<
        PipelineType: 'static,
        S: bevy_ecs::system::IntoSystem<SplitPipelineModuleResource, PipelineType, M> + 'static,
        M,
    > PipelineShaderDescriptor<2, PipelineType, S, M> for SplitShaders
{
    type Descriptor = SplitPipelineModuleDescriptor<PipelineType, S, M>;

    fn into_descriptor(self, system: S) -> SplitPipelineModuleDescriptor<PipelineType, S, M>
    where
        S: IntoSystem<SplitPipelineModuleResource, PipelineType, M> + 'static,
    {
        SplitPipelineModuleDescriptor {
            shaders: self,
            system,
            phantom: PhantomData,
        }
    }
}

pub struct SplitPipelineModuleDescriptor<PipelineType, S, M>
where
    S: IntoSystem<SplitPipelineModuleResource, PipelineType, M> + 'static,
{
    shaders: SplitShaders,
    system: S,
    phantom: PhantomData<(PipelineType, M)>,
}

pub struct SplitPipelineModuleResource {
    pub vertex: Arc<ShaderModule>,
    pub fragment: Arc<ShaderModule>,
}

pub struct SplitPipelineConstructor<PipelineType> {
    system_id: SystemId<SplitPipelineModuleResource, PipelineType>,
}

impl<'a> From<[Arc<ShaderModule>; 2]> for SplitPipelineModuleResource {
    fn from(value: [Arc<ShaderModule>; 2]) -> Self {
        Self {
            vertex: value[0].clone(),
            fragment: value[1].clone(),
        }
    }
}

impl<PipelineType, S, M> PipelineModuleDescriptor<2, PipelineType, S, M>
    for SplitPipelineModuleDescriptor<PipelineType, S, M>
where
    S: IntoSystem<SplitPipelineModuleResource, PipelineType, M> + 'static,
    PipelineType: 'static,
{
    fn as_arr(&self) -> [ShaderHandle; 2] {
        [self.shaders.vertex, self.shaders.fragment]
    }
    fn constructor(
        // system_ids: Vec<SystemId<SplitPipelineModuleResource, PipelineType>>,
        self,
        world: &mut World,
    ) -> impl PipelineConstructor<PipelineType> + 'static {
        SplitPipelineConstructor {
            system_id: world.register_system(self.system),
        }
    }
}

impl<PipelineType: 'static> PipelineConstructor<PipelineType>
    for SplitPipelineConstructor<PipelineType>
{
    fn construct(&mut self, moduels: &[Arc<ShaderModule>], rr: &mut World) -> PipelineType {
        rr.run_system_with_input(
            self.system_id,
            SplitPipelineModuleResource {
                vertex: moduels[0].clone(),
                fragment: moduels[1].clone(),
            },
        )
        .unwrap()
    }
}
