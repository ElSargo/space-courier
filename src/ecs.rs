use bevy_ecs::{
    schedule::{Schedule, ScheduleLabel},
    world::World,
};

pub struct Initailiser {
    pub world: World,
    pub init: Schedule,
    pub update: Schedule,
    pub prepare_buffers: Schedule,
    pub resize: Schedule,
    pub render: Schedule,
}

#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
pub struct UpdateSchedual;
#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
pub struct PrepareBuffersSchedual;
#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
pub struct InitSchedual;
#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
pub struct ResizeSchedual;
#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
pub struct RenderSchedual;

impl Initailiser {
    pub fn new() -> Self {
        let mut init = Schedule::new(InitSchedual);
        init.set_executor_kind(bevy_ecs::schedule::ExecutorKind::Simple);
        let mut resize = Schedule::new(ResizeSchedual);
        resize.set_executor_kind(bevy_ecs::schedule::ExecutorKind::MultiThreaded);
        let mut render = Schedule::new(RenderSchedual);
        render.set_executor_kind(bevy_ecs::schedule::ExecutorKind::MultiThreaded);

        Self {
            world: World::new(),
            update: Schedule::new(UpdateSchedual),
            prepare_buffers: Schedule::new(PrepareBuffersSchedual),
            init,
            resize,
            render,
        }
    }

    pub fn finalize(self) -> World {
        let Self {
            mut world,
            init,
            update,
            prepare_buffers,
            resize,
            render,
        } = self;
        world.add_schedule(init);
        world.add_schedule(render);
        world.add_schedule(resize);

        world.add_schedule(update);
        world.add_schedule(prepare_buffers);

        world
    }
}
