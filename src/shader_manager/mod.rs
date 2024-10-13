mod loader;
mod pipeline_shader_resource;
mod solid_pipeline;
mod split_pipeline;
mod unified_pipeline;

use bevy_ecs::{component::Component, system::Resource, world::World};
use crossbeam_channel::{unbounded, Receiver, Sender};
use notify::{
    event::{AccessMode, MetadataKind},
    recommended_watcher, Event, RecommendedWatcher, Watcher,
};
use parking_lot::Mutex;
use pollster::FutureExt;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    sync::{Arc, RwLock, RwLockReadGuard},
    thread,
    time::{self, Instant},
};
use wgpu::{Device, ShaderModule, ShaderModuleDescriptor};

pub use self::{
    loader::ShaderLoader,
    pipeline_shader_resource::{PipelineConstructor, PipelineModuleDescriptor},
    solid_pipeline::{SolidPipelineModuleResource, SolidPipelineModuleResourceExtras, SolidShader},
    split_pipeline::{SplitPipelineModuleResource, SplitShaders},
    unified_pipeline::{UnifiedPipelineModuleResource, UnifiedShaders},
};

type Adder<PipelineType> =
    Box<dyn FnOnce(&mut PipelineManager<PipelineType>, &mut World) + Send + Sync>;

pub struct ShaderManager {
    _file_whatcher: RecommendedWatcher,
    compiled_shaders_queue: Receiver<(ShaderModule, ShaderHandle)>,
    shaders: HashMap<ShaderHandle, StoredShader>,
    compute_manager: PipelineManager<wgpu::ComputePipeline>,
    render_manager: PipelineManager<wgpu::RenderPipeline>,
}

struct PipelineManager<PipelineType> {
    creation_queue: Receiver<Adder<PipelineType>>,
    pipelines_requirements: BTreeMap<ShaderHandle, Vec<PipelineHandle>>,
    pipline_reasoureces: BTreeMap<PipelineHandle, Vec<ShaderHandle>>,
    pipeline_connstructors: BTreeMap<PipelineHandle, Box<dyn PipelineConstructor<PipelineType>>>,
    pipelines: Arc<RwLock<BTreeMap<PipelineHandle, PipelineType>>>,
    pipelines_to_init: BTreeSet<PipelineHandle>,
}

#[derive(Resource)]
pub struct PipelinesView<PipelineType>(Arc<RwLock<BTreeMap<PipelineHandle, PipelineType>>>);

impl<T> Clone for PipelinesView<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub struct PipelineView<'a, PipelineType> {
    guard: RwLockReadGuard<'a, BTreeMap<PipelineHandle, PipelineType>>,
    handle: PipelineHandle,
}

impl<'a, PipelineType> Deref for PipelineView<'a, PipelineType> {
    type Target = PipelineType;

    fn deref(&self) -> &Self::Target {
        self.guard.get(&self.handle).map(|sp|sp).expect(
            "\
Pipeline dereferenced without being created, something is broken with the shader manager, 
this shouldn't happen since the manager validates that the pipeline exists before returning the view"
        )
    }
}

impl<PipelineType: 'static> PipelineManager<PipelineType> {
    fn new() -> (
        Self,
        PipelinesView<PipelineType>,
        Sender<Adder<PipelineType>>,
    ) {
        let pipelines = Arc::new(RwLock::new(BTreeMap::new()));
        let (send, rcv) = unbounded();
        (
            Self {
                creation_queue: rcv,
                pipelines_requirements: BTreeMap::new(),
                pipelines_to_init: BTreeSet::new(),

                pipelines: pipelines.clone(),

                pipeline_connstructors: BTreeMap::new(),

                pipline_reasoureces: BTreeMap::new(),
            },
            PipelinesView(pipelines),
            send,
        )
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
        shaders: &HashMap<ShaderHandle, StoredShader>,
        rr: &mut World,
    ) {
        self.pipelines_to_init.remove(pipline_handle);
        if let Some(reasources) = self.pipline_reasoureces.get(pipline_handle) {
            let mut compiled_shaders = Vec::with_capacity(reasources.len());
            for sh in reasources {
                if let Some(shader) = shaders.get(sh) {
                    compiled_shaders.push(shader);
                } else {
                    return;
                }
            }
            if let Some(constructor) = self.pipeline_connstructors.get_mut(pipline_handle) {
                let new_pipeline = constructor.construct(
                    compiled_shaders
                        .iter()
                        .map(|s| s.module.clone())
                        .collect::<Vec<_>>()
                        .as_slice(),
                    rr,
                );
                let mut guard = self.pipelines.write().unwrap();
                guard.insert(*pipline_handle, new_pipeline);
            }
        }
    }

    fn recompile_dependants(
        &mut self,
        shader: ShaderHandle,
        shaders: &HashMap<ShaderHandle, StoredShader>,
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

#[derive(Component, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Clone, Copy)]
pub struct PipelineHandle {
    id: u64,
}

#[macro_export]
macro_rules! log_expand {
    (  $txt:expr, $col:ident ) => {
        &color_eyre::owo_colors::OwoColorize::$col(&format!($txt))
    };
    (  $txt:expr, $col:ident, $($a:expr, $b:ident), + ) => {
        &format!("{}{}", log_expand!($txt, $col ), log_expand!($($a, $b), +))
    };
    ( $msg:expr ) => { &format!($msg) };

}

#[macro_export]
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

impl ShaderManager {
    pub fn new(
        device: Arc<Device>,
    ) -> Result<
        (
            Self,
            ShaderLoader,
            PipelinesView<wgpu::RenderPipeline>,
            PipelinesView<wgpu::ComputePipeline>,
        ),
        notify::Error,
    > {
        let (compile_queue_sender, compile_queue_rcv) = unbounded();
        let (to_filter_sender, to_filter_rcv) = unbounded();

        let (compiled_shaders_queue_sender, compiled_shaders_queue) = unbounded();

        std::thread::spawn(move || {
            compile_shaders(
                compile_queue_rcv,
                device.clone(),
                compiled_shaders_queue_sender,
            )
        });

        let (compute_manager, compute_view, compute_queue_send) = PipelineManager::new();
        let (render_manager, render_view, render_queue_send) = PipelineManager::new();

        Ok((
            Self {
                _file_whatcher: begin_whatching_fs(to_filter_rcv, compile_queue_sender),
                compiled_shaders_queue,
                shaders: HashMap::new(),
                compute_manager,
                render_manager,
            },
            ShaderLoader::new(to_filter_sender, compute_queue_send, render_queue_send),
            render_view,
            compute_view,
        ))
    }

    pub fn construct_piplines(&mut self, rr: &mut World) {
        while let Ok((module, shader_handle)) = self.compiled_shaders_queue.try_recv() {
            match self.shaders.get_mut(&shader_handle) {
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
                shaders: &HashMap<ShaderHandle, StoredShader>,
                rr: &mut World,
                shader_handle: ShaderHandle,
            ) {
                while let Ok(f) = render_manager.creation_queue.try_recv() {
                    f(render_manager, rr);
                }
                render_manager.recompile_dependants(shader_handle, &shaders, rr);
                for pipeline_handle in render_manager.pipelines_to_init.clone().iter() {
                    render_manager.proccess_pipline(&pipeline_handle, &shaders, rr);
                }
            }

            process_pipelines(&mut self.render_manager, &self.shaders, rr, shader_handle);
            process_pipelines(&mut self.compute_manager, &self.shaders, rr, shader_handle);
        }
    }
}

// Is safe since Instant boils down to a time spec
const UNIX_EPOCH: Instant = unsafe { std::mem::transmute(time::UNIX_EPOCH) };

fn begin_whatching_fs(
    to_unfilter: Receiver<(PathBuf, ShaderHandle)>,
    to_compile: Sender<(PathBuf, ShaderHandle)>,
) -> RecommendedWatcher {
    let compile_time_map = Arc::new(Mutex::new(HashMap::new()));
    let (a, b) = (to_compile.clone(), compile_time_map.clone());
    thread::spawn(move || add_filters(to_unfilter.clone(), a, b));

    let mut whatcher = recommended_watcher(create_fs_event_handler(compile_time_map, to_compile))
        .expect("Whatcher failed to spawn: can't load anything, exiting");

    whatcher
        .watch(&Path::new("./src"), notify::RecursiveMode::Recursive)
        .expect("Can't whatch paths");

    whatcher
}

fn create_fs_event_handler(
    compile_time_map: Arc<Mutex<HashMap<ShaderHandle, Instant>>>,
    to_compile: Sender<(PathBuf, ShaderHandle)>,
) -> impl Fn(Result<Event, notify::Error>) {
    move |event| {
        match event {
            Ok(Event { kind, paths, .. }) if fs_event_is_relevent(kind) => {
                send_paths(gen_path_ids(paths), &compile_time_map, &to_compile);
            }

            Err(ref e) => eprintln!("Whatcher error: {e}"),

            // Ignore everything else;
            _ => {}
        }
    }
}

fn gen_path_ids(paths: Vec<PathBuf>) -> Vec<(PathBuf, ShaderHandle)> {
    let paths = paths
        .into_iter()
        .flat_map(|p| {
            p.canonicalize().map(|path| {
                let id = path_to_shader_id(&path);
                (path, id)
            })
        })
        .collect();
    paths
}

fn add_filters(
    to_unfilter: Receiver<(PathBuf, ShaderHandle)>,
    to_compile: Sender<(PathBuf, ShaderHandle)>,
    compile_time_map: Arc<Mutex<HashMap<ShaderHandle, std::time::Instant>>>,
) {
    loop {
        let mut p = vec![to_unfilter.recv().unwrap()];
        while let Ok(i) = to_unfilter.try_recv() {
            p.push(i);
        }
        {
            let mut lk = compile_time_map.lock();
            for (_, id) in p.iter() {
                lk.entry(*id).or_insert(UNIX_EPOCH);
            }
        }

        send_paths(
            // Avoid allocating unsessisary vecs by only creating a new one when needed
            p,
            &compile_time_map,
            &to_compile,
        );
    }
}
fn send_paths(
    paths: Vec<(PathBuf, ShaderHandle)>,
    compile_time_map: &Mutex<HashMap<ShaderHandle, std::time::Instant>>,
    cqs: &Sender<(PathBuf, ShaderHandle)>,
) {
    let recived_on = std::time::Instant::now();
    let mut lk = compile_time_map.lock();
    for (path, id) in paths.into_iter() {
        let Some(last_time) = lk.get_mut(&id) else {
            continue;
        };

        if recived_on.duration_since(*last_time).as_millis() < 100 {
            continue;
        }
        *last_time = recived_on;
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
    while let Ok(id) = incomming.recv() {
        let (outgoing, device) = (outgoing.clone(), device.clone());
        std::thread::spawn(move || compile_shader(id, device, outgoing));
    }
}

fn compile_shader(
    (path, id): (PathBuf, ShaderHandle),
    device: Arc<Device>,
    outgoing: Sender<(ShaderModule, ShaderHandle)>,
) {
    log!("Compiling ", bright_yellow, "{path:#?}", bright_purple);
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
            outgoing.send((shader, id)).unwrap();
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

impl<PipelineType> PipelinesView<PipelineType> {
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
