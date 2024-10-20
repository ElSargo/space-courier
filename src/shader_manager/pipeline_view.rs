use crate::prelude::*;

#[derive(Resource)]
pub struct PipelinesView<PipelineType>(pub(super) BTreeMap<PipelineHandle, PipelineType>);

impl<PipelineType> Default for PipelinesView<PipelineType> {
    fn default() -> Self {
        Self::new()
    }
}

impl<PipelineType> PipelinesView<PipelineType> {
    pub fn get(&self, handle: PipelineHandle) -> Option<&PipelineType> {
        self.0.get(&handle)
    }

    pub fn new() -> Self {
        Self(BTreeMap::new())
    }
}
