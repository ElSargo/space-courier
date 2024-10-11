use std::{marker::PhantomData, ops::Deref, sync::Arc};

use crate::shader_manager::ShaderHandle;
use bevy_ecs::{
    system::{IntoSystem, Resource, SystemId},
    world::World,
};
use wgpu::ShaderModule;

use super::{
    pipeline_shader_resource::PipelineShaderDescriptor, PipelineConstructor,
    PipelineModuleDescriptor,
};

pub struct UnifiedShaders {
    pub handle: ShaderHandle,
}

impl<
        PipelineType: 'static,
        S: bevy_ecs::system::IntoSystem<UnifiedPipelineModuleResource, PipelineType, M> + 'static,
        M,
    > PipelineShaderDescriptor<1, PipelineType, S, M> for UnifiedShaders
{
    type Descriptor = UnifiedPipelineModuleDescriptor<PipelineType, S, M>;

    fn into_descriptor(self, system: S) -> UnifiedPipelineModuleDescriptor<PipelineType, S, M> {
        UnifiedPipelineModuleDescriptor {
            handle: self.handle,
            system,
            phantom: PhantomData,
        }
    }
}

pub struct UnifiedPipelineModuleDescriptor<PipelineType, S, M>
where
    S: IntoSystem<UnifiedPipelineModuleResource, PipelineType, M> + 'static,
{
    handle: ShaderHandle,
    system: S,
    phantom: PhantomData<(PipelineType, M)>,
}

#[derive(Resource)]
pub struct UnifiedPipelineModuleResource {
    pub module: Arc<ShaderModule>,
}

pub struct UnifiedPipelineConstructor<PipelineType> {
    system_id: SystemId<UnifiedPipelineModuleResource, PipelineType>,
}

impl<
        PipelineType: 'static,
        S: bevy_ecs::system::IntoSystem<UnifiedPipelineModuleResource, PipelineType, M>,
        M,
    > PipelineModuleDescriptor<1, PipelineType, S, M>
    for UnifiedPipelineModuleDescriptor<PipelineType, S, M>
{
    fn as_arr(&self) -> [ShaderHandle; 1] {
        [self.handle]
    }
    fn constructor(self, world: &mut World) -> impl PipelineConstructor<PipelineType> + 'static {
        UnifiedPipelineConstructor {
            system_id: world.register_system(self.system),
        }
    }
}

impl<PipelineType: 'static> PipelineConstructor<PipelineType>
    for UnifiedPipelineConstructor<PipelineType>
{
    fn construct(&mut self, moduels: &[Arc<ShaderModule>], rr: &mut World) -> PipelineType {
        rr.run_system_with_input(
            self.system_id,
            UnifiedPipelineModuleResource {
                module: moduels[0].clone(),
            },
        )
        .unwrap()
    }
}

impl Deref for UnifiedPipelineModuleResource {
    type Target = ShaderModule;

    fn deref(&self) -> &Self::Target {
        &self.module
    }
}
