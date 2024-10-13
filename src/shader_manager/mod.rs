mod pipeline_shader_resource;
mod solid_pipeline;
mod split_pipeline;
mod unified_pipeline;
use bevy_ecs::{component::Component, system::Resource, world::World};

use notify::{
    event::{AccessMode, MetadataKind},
    recommended_watcher, Event, RecommendedWatcher, RecursiveMode, Watcher,
};
use pollster::FutureExt;
use rand::random;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    sync::{Arc, RwLock, RwLockReadGuard},
};

// use std::sync::mpsc::{channel, Receiver, Sender};
use crossbeam_channel::{unbounded, Receiver, Sender};

use parking_lot::Mutex;
use wgpu::{Device, ShaderModule, ShaderModuleDescriptor};

use self::pipeline_shader_resource::PipelineShaderDescriptor;
pub use self::{
    pipeline_shader_resource::{PipelineConstructor, PipelineModuleDescriptor},
    solid_pipeline::{SolidPipelineModuleResource, SolidPipelineModuleResourceExtras, SolidShader},
    split_pipeline::{SplitPipelineModuleResource, SplitShaders},
    unified_pipeline::{UnifiedPipelineModuleResource, UnifiedShaders},
};

type Adder<PipelineType> =
    Box<dyn FnOnce(&mut PipelineManager<PipelineType>, &mut World) + Send + Sync>;

pub struct ShaderManager {
    compiled_shaders_queue: Receiver<(ShaderModule, ShaderHandle)>,
    shaders: ShaderStorage,

    compute_manager: PipelineManager<wgpu::ComputePipeline>,
    compute_creation_queue: Receiver<Adder<wgpu::ComputePipeline>>,

    render_manager: PipelineManager<wgpu::RenderPipeline>,
    render_creation_queue: Receiver<Adder<wgpu::RenderPipeline>>,
}

struct PipelineManager<PipelineType> {
    pipelines_requirements: BTreeMap<ShaderHandle, Vec<PipelineHandle>>,
    pipline_reasoureces: BTreeMap<PipelineHandle, Vec<ShaderHandle>>,
    pipeline_connstructors: BTreeMap<PipelineHandle, Box<dyn PipelineConstructor<PipelineType>>>,
    pipelines: Arc<RwLock<BTreeMap<PipelineHandle, StoredPipeline<PipelineType>>>>,
    pipelines_to_init: BTreeSet<PipelineHandle>,
}

#[derive(Resource, Clone)]
pub struct ShaderManagerCtrl {
    whatcher: Arc<Mutex<RecommendedWatcher>>,
    compile_queue_sender: Sender<(PathBuf, ShaderHandle)>,
    compute_creation_queue: Sender<Adder<wgpu::ComputePipeline>>,
    render_creation_queue: Sender<Adder<wgpu::RenderPipeline>>,
}

#[derive(Resource)]
pub struct PipelinesView<PipelineType>(
    Arc<RwLock<BTreeMap<PipelineHandle, StoredPipeline<PipelineType>>>>,
);

impl<T> Clone for PipelinesView<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub struct PipelineView<'a, PipelineType> {
    guard: RwLockReadGuard<'a, BTreeMap<PipelineHandle, StoredPipeline<PipelineType>>>,
    handle: PipelineHandle,
}

impl<'a, PipelineType> Deref for PipelineView<'a, PipelineType> {
    type Target = PipelineType;

    fn deref(&self) -> &Self::Target {
        self.guard.get(&self.handle).map(|sp|&sp.render_pipeline).expect(
            "\
Pipeline dereferenced without being created, something is broken with the shader manager, 
this shouldn't happen since the manager validates that the pipeline exists before returning the view"
        )
    }
}

impl<PipelineType: 'static> PipelineManager<PipelineType> {
    fn new(pipelines: Arc<RwLock<BTreeMap<PipelineHandle, StoredPipeline<PipelineType>>>>) -> Self {
        Self {
            pipelines_requirements: BTreeMap::new(),
            pipelines_to_init: BTreeSet::new(),

            pipelines,

            pipeline_connstructors: BTreeMap::new(),

            pipline_reasoureces: BTreeMap::new(),
        }
    }

    pub fn add_pipeline<const N: usize, Descriptor, System, M>(
        &mut self,
        descriptor: Descriptor,
        pipeline_handle: PipelineHandle,
        rr: &mut World,
    ) where
        Descriptor: PipelineModuleDescriptor<N, PipelineType, System, M>, // Descriptor: PipelineModuleDescriptor<N, PipelineType, System, M> + Send + Sync,
    {
        for shader_handle in descriptor.as_arr() {
            self.pipelines_requirements
                .entry(shader_handle)
                .or_insert(vec![])
                .push(pipeline_handle)
        }

        self.pipline_reasoureces
            .insert(pipeline_handle, descriptor.as_arr().to_vec());
        self.pipeline_connstructors
            .insert(pipeline_handle, Box::new(descriptor.constructor(rr)));
    }

    fn proccess_pipline(
        &mut self,
        pipline_handle: &PipelineHandle,
        shaders: &ShaderStorage,
        rr: &mut World,
    ) {
        self.pipelines_to_init.remove(pipline_handle);
        if let Some(reasources) = self.pipline_reasoureces.get(pipline_handle) {
            let mut compiled_shaders = Vec::with_capacity(reasources.len());
            for sh in reasources {
                if let Some(shader) = shaders.get(*sh) {
                    compiled_shaders.push(shader);
                } else {
                    return;
                }
            }

            let shader_generations = compiled_shaders
                .iter()
                .map(|s| s.generation)
                .collect::<Vec<_>>();

            let should_recompile =
                if let Some(cached_pipeline) = self.pipelines.read().unwrap().get(pipline_handle) {
                    shader_generations != cached_pipeline.generations
                } else {
                    true
                };

            if should_recompile {
                if let Some(constructor) = self.pipeline_connstructors.get_mut(pipline_handle) {
                    // let shader_modules = compiled_shaders.iter().map(|s| &s.module).collect();
                    let new_pipeline = constructor.construct(
                        compiled_shaders
                            .iter()
                            .map(|s| s.module.clone())
                            .collect::<Vec<_>>()
                            .as_slice(),
                        rr,
                    );
                    // = self
                    //     .gpu_device
                    //     .create_render_pipeline(&new_pipeline_descriptor);
                    let mut guard = self.pipelines.write().unwrap();
                    match guard.get_mut(pipline_handle) {
                        Some(stored_pipeline) => {
                            stored_pipeline.render_pipeline = new_pipeline;
                            stored_pipeline.generations = shader_generations;
                        }
                        None => {
                            guard.insert(
                                *pipline_handle,
                                StoredPipeline {
                                    render_pipeline: new_pipeline,
                                    generations: shader_generations,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    fn recompile_dependants(
        &mut self,
        shader: ShaderHandle,
        shaders: &ShaderStorage,
        rr: &mut World,
    ) {
        if let Some(pipelines_to_reconstruct) = self.pipelines_requirements.get(&shader).cloned() {
            for pipline_handle in pipelines_to_reconstruct {
                self.proccess_pipline(&pipline_handle, &shaders, rr);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShaderHandle {
    id: u64,
}

struct StoredShader {
    module: Arc<ShaderModule>,
    generation: u64,
}

struct StoredPipeline<T> {
    render_pipeline: T,
    generations: Vec<u64>,
}

#[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Clone, Copy)]
pub struct PipelineHandle {
    id: u64,
}

macro_rules! log_expand {
    (  $txt:expr, $col:ident ) => {
        &color_eyre::owo_colors::OwoColorize::$col(&format!($txt))
    };
    (  $txt:expr, $col:ident, $($a:expr, $b:ident), + ) => {
        &format!("{}{}", log_expand!($txt, $col ), log_expand!($($a, $b), +))
    };
    ( $msg:expr ) => { &format!($msg) };

}

macro_rules! log {
    ($($t:tt), +) => {
        println!(
            "{}{}",
            &color_eyre::owo_colors::OwoColorize::bold(&color_eyre::owo_colors::OwoColorize::blue(
                &"[ShaderManager]: "
            )),
            log_expand!($($t), +)
        )
    }
}

pub struct ShaderStorage {
    shaders: HashMap<ShaderHandle, StoredShader>,
}
impl ShaderStorage {
    fn new() -> Self {
        Self {
            shaders: HashMap::new(),
        }
    }

    fn get_mut<'a>(&'a mut self, handle: ShaderHandle) -> Option<&'a mut StoredShader> {
        self.shaders.get_mut(&handle)
    }

    fn insert(&mut self, handle: ShaderHandle, item: StoredShader) {
        self.shaders.insert(handle, item);
    }

    fn get<'a>(&'a self, handle: ShaderHandle) -> Option<&'a StoredShader> {
        self.shaders.get(&handle)
    }
}

impl ShaderManager {
    pub fn new(
        device: Arc<Device>,
    ) -> Result<
        (
            Self,
            ShaderManagerCtrl,
            PipelinesView<wgpu::RenderPipeline>,
            PipelinesView<wgpu::ComputePipeline>,
        ),
        notify::Error,
    > {
        let (compile_queue_sender, rcv) = unbounded();
        let cqs = compile_queue_sender.clone();
        let mut compile_time_map = HashMap::new();

        let (re_whatch_send, re_whatch_rcv) = unbounded();

        let whatcher = Arc::new(Mutex::new(recommended_watcher(move |event| {
            println!("Recived event: {:#?}", event);
            match event {
                Ok(Event { kind, paths, .. }) if fs_event_is_relevent(kind) => {
                    send_paths(paths, &mut compile_time_map, &cqs);
                }
                Ok(Event {
                    kind: notify::event::EventKind::Remove(_),
                    paths,
                    ..
                }) => {
                    re_whatch_send.send(paths).unwrap();
                }
                Err(ref e) => eprintln!("Whatcher error: {e}"),
                _ => {}
            }
        })?));

        let wt = whatcher.clone();
        std::thread::spawn(move || re_whatch_paths(re_whatch_rcv, wt));

        let (compiled_shaders_queue_sender, compiled_shaders_queue) = unbounded();

        std::thread::spawn(move || {
            compile_shaders(rcv, device.clone(), compiled_shaders_queue_sender)
        });

        let (compute_queue_send, compute_queue_rcv) = unbounded();
        let (render_queue_send, render_queue_rcv) = unbounded();

        let render_pipelines = Arc::new(RwLock::new(BTreeMap::new()));
        let compute_pipelines = Arc::new(RwLock::new(BTreeMap::new()));

        Ok((
            Self {
                compiled_shaders_queue,
                shaders: ShaderStorage::new(), // constructors: HashMap::new(),
                compute_manager: PipelineManager::new(compute_pipelines.clone()),
                compute_creation_queue: compute_queue_rcv,
                render_manager: PipelineManager::new(render_pipelines.clone()),
                render_creation_queue: render_queue_rcv,
            },
            ShaderManagerCtrl {
                whatcher,
                compile_queue_sender,
                compute_creation_queue: compute_queue_send,
                render_creation_queue: render_queue_send,
            },
            PipelinesView(render_pipelines),
            PipelinesView(compute_pipelines),
        ))
    }

    pub fn construct_piplines(&mut self, rr: &mut World) {
        while let Ok((module, shader_handle)) = self.compiled_shaders_queue.try_recv() {
            match self.shaders.get_mut(shader_handle) {
                Some(ss) => {
                    ss.module = module.into();
                    ss.generation += 1;
                }
                None => {
                    self.shaders.insert(
                        shader_handle,
                        StoredShader {
                            module: module.into(),
                            generation: 0,
                        },
                    );
                }
            }

            fn process_pipelines<PipelineType: 'static>(
                render_manager: &mut PipelineManager<PipelineType>,
                render_creation_queue: &Receiver<
                    Box<dyn FnOnce(&mut PipelineManager<PipelineType>, &mut World) + Send + Sync>,
                >,
                shaders: &ShaderStorage,
                rr: &mut World,
                shader_handle: ShaderHandle,
            ) {
                while let Ok(f) = render_creation_queue.try_recv() {
                    f(render_manager, rr);
                }
                render_manager.recompile_dependants(shader_handle, &shaders, rr);
                for pipeline_handle in render_manager.pipelines_to_init.clone().iter() {
                    render_manager.proccess_pipline(&pipeline_handle, &shaders, rr);
                }
            }

            process_pipelines(
                &mut self.render_manager,
                &self.render_creation_queue,
                &self.shaders,
                rr,
                shader_handle,
            );
            process_pipelines(
                &mut self.compute_manager,
                &self.compute_creation_queue,
                &self.shaders,
                rr,
                shader_handle,
            );
        }
    }
}

fn re_whatch_paths(
    re_whatch_rcv: Receiver<Vec<PathBuf>>,
    whatcher: Arc<Mutex<notify::INotifyWatcher>>,
) {
    while let Ok(paths) = re_whatch_rcv.recv() {
        {
            let mut whatcher = whatcher.lock();
            for path in paths {
                whatcher.watch(&path, RecursiveMode::NonRecursive).unwrap();
            }
        }
    }
}

fn send_paths(
    paths: Vec<PathBuf>,
    compile_time_map: &mut HashMap<ShaderHandle, std::time::Instant>,
    cqs: &Sender<(PathBuf, ShaderHandle)>,
) {
    let recived_on = std::time::Instant::now();
    for path in paths {
        println!("Sending: {:#?}", path);

        let id = path_to_shader_id(&path);

        if let Some(false) = compile_time_map
            .get(&id)
            .map(|last_time| recived_on.duration_since(*last_time).as_millis() > 100)
        {
            println!("Skipping {:#?}", path);
            continue;
        }

        compile_time_map.insert(id, recived_on);

        cqs.send((path, id))
            .expect("Unable to send fs-msg from whatcher to shader loader, channel error");
    }
}

fn fs_event_is_relevent(kind: notify::EventKind) -> bool {
    matches!(
        kind,
        notify::EventKind::Access(notify::event::AccessKind::Close(AccessMode::Write))
            | notify::EventKind::Modify(
                notify::event::ModifyKind::Data(notify::event::DataChange::Any)
                    | notify::event::ModifyKind::Metadata(MetadataKind::WriteTime)
                    | notify::event::ModifyKind::Name(notify::event::RenameMode::From),
            )
    )
}

/// Stable hash
fn path_to_shader_id(path: &PathBuf) -> ShaderHandle {
    use std::hash::Hasher;
    let mut hasher = seahash::SeaHasher::default();
    hasher.write(path.as_os_str().as_bytes());
    ShaderHandle {
        id: hasher.finish(),
    }
}

fn compile_shaders(
    incomming: Receiver<(PathBuf, ShaderHandle)>,
    device: Arc<Device>,
    outgoing: Sender<(ShaderModule, ShaderHandle)>,
) {
    while let Ok((path, handle)) = incomming.recv() {
        log!("Compiling ", bright_yellow, "{path:#?}", bright_purple);
        let (outgoing, device) = (outgoing.clone(), device.clone());
        // handle.map(| | )
        std::thread::spawn(move || compile_shader(path, device, handle, outgoing));
    }
}

fn compile_shader(
    path: PathBuf,
    device: Arc<Device>,
    handle: ShaderHandle,
    outgoing: Sender<(ShaderModule, ShaderHandle)>,
) {
    match std::fs::read_to_string(&path) {
        Ok(source) => {
            let shader;
            {
                // Catch compile error scope
                device.push_error_scope(wgpu::ErrorFilter::Validation);
                shader = device.create_shader_module(ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Owned(source)),
                });

                if let Some(e) = device.pop_error_scope().block_on() {
                    print_complile_error(e);
                    return;
                }
            }

            log!("Compiled ", green, "{path:#?}", bright_purple);
            outgoing.send((shader, handle)).unwrap();
        }
        Err(e) => eprintln!("{e}"),
    }
}

fn print_complile_error(e: wgpu::Error) {
    match e {
        wgpu::Error::OutOfMemory { source } => eprintln!("{source:#?}"),
        wgpu::Error::Validation { description, .. } => {
            eprintln!("{description}")
        }
        wgpu::Error::Internal {
            source: _,
            description: _,
        } => todo!(),
    }
}

#[macro_export]
macro_rules! load_shader {
    ( $sm:expr, $path:expr) => {{
        let mut path: std::path::PathBuf = file!().into();
        path.pop();
        path.push($path);
        $sm.load_shader(path)
    }};
}

impl ShaderManagerCtrl {
    pub fn load_shader(&self, path: impl Into<PathBuf>) -> ShaderHandle {
        let path = path.into();
        let path = path
            .canonicalize()
            .expect(&format!("Unable to resolve file path {path:#?}"));

        log!(
            "Sent ",
            bright_red,
            "{path:#?}",
            bright_purple,
            " to queue",
            white
        );

        let handle = path_to_shader_id(&path);

        self.compile_queue_sender
            .send((path.clone(), handle))
            .expect("Unable to send shader to load queue");

        let w = self.whatcher.clone();
        std::thread::spawn(move || {
            w.lock()
                .watch(&path, notify::RecursiveMode::NonRecursive)
                .expect("File whatcher failed to accept new path");
        });

        handle
    }

    pub fn add_render_pipeline<'a, const N: usize, System, Shaders, M>(
        &'a self,
        shaders: Shaders,
        system: System, // connstructor: impl IntoSystem<OutputResource, wgpu::RenderPipeline, M> + Send + Sync + 'static,
    ) -> PipelineHandle
    where
        Shaders: PipelineShaderDescriptor<N, wgpu::RenderPipeline, System, M>
            + std::marker::Send
            + std::marker::Sync,
        <Shaders as PipelineShaderDescriptor<N, wgpu::RenderPipeline, System, M>>::Descriptor: Send,
        <Shaders as PipelineShaderDescriptor<N, wgpu::RenderPipeline, System, M>>::Descriptor: Sync,
        <Shaders as PipelineShaderDescriptor<N, wgpu::RenderPipeline, System, M>>::Descriptor:
            'static,
    {
        let pipeline_handle = PipelineHandle { id: random() };

        let descriptor = Shaders::into_descriptor(shaders, system);

        self.render_creation_queue
            .send(Box::new(
                move |render_manager: &mut PipelineManager<wgpu::RenderPipeline>, rr| {
                    render_manager.add_pipeline(descriptor, pipeline_handle, rr)
                },
            ))
            .expect("Render creatation queue closed for some reason, this shouldn't happen");
        pipeline_handle
    }

    pub fn add_compute_pipeline<'a, const N: usize, System, Shaders, M>(
        &'a self,
        shaders: Shaders,
        system: System, // connstructor: impl IntoSystem<OutputResource, wgpu::ComputePipeline, M> + Send + Sync + 'static,
    ) -> PipelineHandle
    where
        Shaders: PipelineShaderDescriptor<N, wgpu::ComputePipeline, System, M>
            + std::marker::Send
            + std::marker::Sync,
        <Shaders as PipelineShaderDescriptor<N, wgpu::ComputePipeline, System, M>>::Descriptor:
            Send,
        <Shaders as PipelineShaderDescriptor<N, wgpu::ComputePipeline, System, M>>::Descriptor:
            Sync,
        <Shaders as PipelineShaderDescriptor<N, wgpu::ComputePipeline, System, M>>::Descriptor:
            'static,
    {
        let pipeline_handle = PipelineHandle { id: random() };

        let descriptor = Shaders::into_descriptor(shaders, system);

        self.compute_creation_queue
            .send(Box::new(
                move |compute_manager: &mut PipelineManager<wgpu::ComputePipeline>, rr| {
                    compute_manager.add_pipeline(descriptor, pipeline_handle, rr)
                },
            ))
            .expect("Compute creatation queue closed for some reason, this shouldn't happen");
        pipeline_handle
    }
}

impl<PipelineType> PipelinesView<PipelineType> {
    // pub fn get_render_pipeline(
    //     &self,
    //     pipeline_handle: &PipelineHandle,
    // ) -> Option<PipelineView<RenderPipeline>> {
    //     self.render_manager.get_pipeline(pipeline_handle)
    // }

    // pub fn get_compute_pipeline(
    //     &self,
    //     pipeline_handle: &PipelineHandle,
    // ) -> Option<PipelineView<ComputePipeline>> {
    //     self.compute_manager.get_pipeline(pipeline_handle)
    // }

    pub fn get(&self, handle: &PipelineHandle) -> Option<PipelineView<PipelineType>> {
        let guard = self.0.read().unwrap();
        guard.contains_key(handle).then_some(PipelineView {
            guard,
            handle: *handle,
        })
    }
}

#[derive(Resource, Component)]
pub struct Marker<T, Id>(T, PhantomData<Id>);

impl<T, I> Marker<T, I> {
    pub fn new(value: T) -> Self {
        Self(value, PhantomData)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T, I> Deref for Marker<T, I> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, I> DerefMut for Marker<T, I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
