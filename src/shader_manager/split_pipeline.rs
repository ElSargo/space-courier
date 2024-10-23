use super::{pipeline_shader_resource::ShadersD, PipelineConstructor};
use crate::prelude::*;

pub struct SplitShaders {
    pub vertex: ShaderHandle,
    pub fragment: ShaderHandle,
}

pub struct SplitShaderResource {
    pub vertex: Arc<ShaderModule>,
    pub fragment: Arc<ShaderModule>,
}

impl<PipelineType, System, M> ShadersD<PipelineType, System, SplitShaderResource, PipelineType, M>
    for SplitShaders
where
    PipelineType: 'static,
    System: IntoSystem<SplitShaderResource, PipelineType, M> + 'static,
{
    fn handles(&self) -> Box<[ShaderHandle]> {
        Box::new([self.vertex, self.fragment])
    }

    fn constructor(
        self,
        system: System,
        world: &mut World,
    ) -> impl PipelineConstructor<PipelineType> + 'static {
        let id = world.register_system(system);
        move |moduels: &[Arc<ShaderModule>], rr: &mut World| {
            rr.run_system_with_input(
                id,
                SplitShaderResource {
                    vertex: moduels[0].clone(),
                    fragment: moduels[1].clone(),
                },
            )
            .unwrap()
        }
    }
}
