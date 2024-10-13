use crate::{model::DrawModel, prelude::*, renderables::SolidDrawable};
use std::sync::atomic::AtomicUsize;

// Quick hax for removing async from model loader
lazy_static! {
    static ref DUMMY_MODEL: Arc<Model> = Arc::new(Model {
        meshes: vec![],
        materials: vec![],
    });
    static ref MODELS: [arc_swap::ArcSwap<Model>; 20] =
        std::array::from_fn(|_| ArcSwap::new(DUMMY_MODEL.clone()));
    static ref MODEL_INDEX: AtomicUsize = AtomicUsize::new(0);
}

// Useed as a model handel, does not implement copy or clone so can aquire the mutex in deref
#[derive(Debug)]
pub struct ModelHandle(usize);

impl ModelHandle {
    pub fn get(&self) -> Option<Arc<Model>> {
        let handle = MODELS[self.0].load_full();

        (!Arc::ptr_eq(&handle, &DUMMY_MODEL)).then_some(handle)
    }
}

pub fn load_model(
    file_name: &'static str,
    device: Arc<Device>,
    queue: Arc<Queue>,
    layout: Arc<wgpu::BindGroupLayout>,
) -> ModelHandle {
    let idx = MODEL_INDEX.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    std::thread::spawn(move || {
        let model = crate::resources::load_model(file_name, &device, &queue, &layout)
            .block_on()
            .unwrap();

        MODELS[idx].store(model.into());

        // *MODELS[idx].write().unwrap() = Some(model);
    });

    ModelHandle(idx)
}

impl SolidDrawable for ModelHandle {
    type Resource = Model;
    fn draw<'a: 'b, 'b>(
        &'a self,
        render_pass: &mut RenderPass<'b>,
        instances: Range<u32>,
        model: &'a Self::Resource,
    ) {
        render_pass.draw_model_instanced(model, instances);
    }
    fn aquire(&self) -> Option<Arc<Self::Resource>> {
        self.get()
    }
}
