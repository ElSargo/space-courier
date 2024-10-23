use std::sync::Arc;

use crate::shader_manager::ShaderHandle;
use bevy_ecs::{system::IntoSystem, world::World};
use wgpu::ShaderModule;

// System  -|
//          |-> Descriptor -|
// Shaders -|               |-> Constructor( ) -> PipelineType
//                   World -|               |
//                          |-> Resource  --|

pub trait PipelineConstructor<PipelineType>: Send + Sync {
    fn construct(&mut self, moduels: &[Arc<ShaderModule>], rr: &mut World) -> PipelineType;
}

pub trait ShadersD<PipelineType, System, I, O, M>
where
    System: IntoSystem<I, O, M>,
{
    fn handles(&self) -> Box<[ShaderHandle]>;
    fn constructor(
        self,
        system: System,
        world: &mut World,
    ) -> impl PipelineConstructor<PipelineType> + 'static;
}
impl From<ShaderHandle> for [ShaderHandle; 1] {
    fn from(value: ShaderHandle) -> Self {
        [value]
    }
}

impl<PipelineType, F: FnMut(&[Arc<ShaderModule>], &mut World) -> PipelineType + Send + Sync>
    PipelineConstructor<PipelineType> for F
{
    fn construct(&mut self, moduels: &[Arc<ShaderModule>], rr: &mut World) -> PipelineType {
        self(moduels, rr)
    }
}

impl<PipelineType, System, M> ShadersD<PipelineType, System, Arc<ShaderModule>, PipelineType, M>
    for ShaderHandle
where
    System: bevy_ecs::system::IntoSystem<Arc<ShaderModule>, PipelineType, M> + 'static,
    PipelineType: 'static,
{
    fn constructor<'a>(
        self,
        system: System,
        world: &'a mut World,
    ) -> impl PipelineConstructor<PipelineType> + 'static {
        let id = world.register_system(system);

        move |moduels: &[Arc<ShaderModule>], rr: &mut World| {
            rr.run_system_with_input(id, moduels[0].clone()).unwrap()
        }
    }

    fn handles(&self) -> Box<[ShaderHandle]> {
        Box::new([*self])
    }
}
