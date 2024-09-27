use std::sync::Arc;

use crate::shader_manager::ShaderHandle;
use bevy_ecs::world::World;
use wgpu::ShaderModule;

// System  -|
//          |-> Descriptor -|
// Shaders -|               |-> Constructor( ) -> PipelineType
//                   World -|               |
//                          |-> Resource  --|

pub trait PipelineShaderDescriptor<const NUM_MODULES: usize, PipelineType, S, M> {
    type Descriptor: PipelineModuleDescriptor<NUM_MODULES, PipelineType, S, M>;
    fn into_descriptor(self, system: S) -> Self::Descriptor;
}

pub trait PipelineModuleDescriptor<const NUM_MODULES: usize, PipelineType, System, M> {
    fn as_arr(&self) -> [ShaderHandle; NUM_MODULES];
    fn constructor(self, world: &mut World) -> impl PipelineConstructor<PipelineType> + 'static;
}

pub trait PipelineConstructor<PipelineType>: Send + Sync {
    fn construct(&mut self, moduels: &[Arc<ShaderModule>], rr: &mut World) -> PipelineType;
}
