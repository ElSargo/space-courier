use std::{any::Any, ops::Range, sync::Arc};

use bevy_ecs::component::Component;
use wgpu::RenderPass;

use crate::shader_manager::PipelineHandle;

#[derive(Component)]
pub struct SolidRenderable {
    pub pipeline: PipelineHandle,
    pub renderable: Box<dyn AnySolidDrawable + Send + Sync>,
}

pub trait SolidDrawable {
    type Resource: 'static;
    const INSTANCE_BUFFER_SLOT: u32 = 1;
    fn draw<'a: 'b, 'b>(
        &'a self,
        render_pass: &mut RenderPass<'b>,
        instances: Range<u32>,
        asset: &'a Self::Resource,
    );
    fn aquire(&self) -> Option<Arc<Self::Resource>>;
}

pub trait AnySolidDrawable {
    fn instance_buffer_slot(&self) -> u32;

    fn draw<'a: 'b, 'b>(
        &'a self,
        render_pass: &mut RenderPass<'b>,
        instances: Range<u32>,
        asset: &'a dyn Any,
    );
    fn aquire(&self) -> Option<Arc<dyn Any>>;
}

impl<U: SolidDrawable> AnySolidDrawable for U {
    fn instance_buffer_slot(&self) -> u32 {
        Self::INSTANCE_BUFFER_SLOT
    }

    fn draw<'a: 'b, 'b>(
        &'a self,
        render_pass: &mut RenderPass<'b>,
        instances: Range<u32>,
        asset: &'a dyn Any,
    ) {
        if let Some(asset) = asset.downcast_ref::<<U as SolidDrawable>::Resource>() {
            self.draw(render_pass, instances, asset);
        }
    }

    fn aquire(&self) -> Option<Arc<dyn Any>>
where {
        self.aquire().map(|x| x as Arc<dyn Any>)
    }
}
