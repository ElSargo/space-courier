use bevy_ecs::world::World;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::{
    collections::{BTreeMap, HashMap},
    marker::{Send, Sync},
    sync::Arc,
};
use wgpu::ShaderModule;

use super::{
    pipeline_shader_resource::{PipelineConstructor, ShadersD},
    pipeline_view::PipelinesView,
    PipelineHandle, ShaderHandle,
};

pub type Adder<PipelineType> =
    Box<dyn FnOnce(&mut PipelineManager<PipelineType>, &mut World) + Send + Sync>;

pub(super) struct PipelineManager<PipelineType> {
    /// Queue of procedures that register a pipelines constructor on the world
    /// This exists so code can create render pipelines without needing exclusive world access
    pipelines_to_register: Receiver<Adder<PipelineType>>,
    /// Map of pipelines that need to be recompiled when a shader changes
    pipelines_depending_on: BTreeMap<ShaderHandle, Vec<PipelineHandle>>,
    /// Map of the shader modules a pipeline needs to be created
    modules_for_pipeline: BTreeMap<PipelineHandle, Box<[ShaderHandle]>>,
    /// Functions that create the actuall pipelines
    /// These are passed the required modules and can access the world
    pipeline_connstructors: BTreeMap<PipelineHandle, Box<dyn PipelineConstructor<PipelineType>>>,
}

impl<PipelineType: 'static + Send + Sync> PipelineManager<PipelineType> {
    pub(super) fn new() -> (Self, Sender<Adder<PipelineType>>) {
        let (send, rcv) = unbounded();
        (
            Self {
                pipelines_to_register: rcv,
                pipelines_depending_on: BTreeMap::new(),

                pipeline_connstructors: BTreeMap::new(),

                modules_for_pipeline: BTreeMap::new(),
            },
            send,
        )
    }

    pub(super) fn add_pipeline<Shaders, System, I, O, M>(
        &mut self,
        descriptor: Shaders,
        system: System,
        pipeline_handle: PipelineHandle,
        rr: &mut World,
    ) where
        Shaders: ShadersD<PipelineType, System, I, O, M>,
        System: bevy_ecs::system::IntoSystem<I, O, M>,
    {
        let handles = descriptor.handles();
        for shader_handle in &handles {
            self.pipelines_depending_on
                .entry(*shader_handle)
                .or_insert(vec![])
                .push(pipeline_handle)
        }

        self.modules_for_pipeline.insert(pipeline_handle, handles);
        self.pipeline_connstructors.insert(
            pipeline_handle,
            Box::new(descriptor.constructor(system, rr)),
        );
    }

    pub(super) fn construct_pipline(
        &mut self,
        pipline_handle: &PipelineHandle,
        shaders: &HashMap<ShaderHandle, Arc<ShaderModule>>,
        rr: &mut World,
    ) {
        let Some(reasources) = self.modules_for_pipeline.get(pipline_handle) else {
            return;
        };

        let mut compiled_shaders = Vec::with_capacity(reasources.len());
        for sh in reasources {
            let Some(shader) = shaders.get(sh) else {
                return;
            };
            compiled_shaders.push(shader);
        }

        let Some(constructor) = self.pipeline_connstructors.get_mut(pipline_handle) else {
            return;
        };

        let new_pipeline = constructor.construct(
            compiled_shaders
                .iter()
                .map(|s| (**s).clone())
                .collect::<Vec<_>>()
                .as_slice(),
            rr,
        );
        rr.resource_mut::<PipelinesView<PipelineType>>()
            .0
            .insert(*pipline_handle, new_pipeline);
    }

    pub(super) fn recompile_dependants(
        &mut self,
        shader: ShaderHandle,
        shaders: &HashMap<ShaderHandle, Arc<ShaderModule>>,
        rr: &mut World,
    ) {
        if let Some(pipelines_to_reconstruct) = self.pipelines_depending_on.get(&shader).cloned() {
            for pipline_handle in pipelines_to_reconstruct {
                self.construct_pipline(&pipline_handle, &shaders, rr);
            }
        }
    }

    pub(super) fn process_pipelines(
        &mut self,
        shaders: &HashMap<ShaderHandle, Arc<ShaderModule>>,
        rr: &mut World,
        shader_handle: ShaderHandle,
    ) {
        while let Ok(f) = self.pipelines_to_register.try_recv() {
            f(self, rr);
        }
        self.recompile_dependants(shader_handle, &shaders, rr);
    }
}
