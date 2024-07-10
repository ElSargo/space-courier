use std::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};

trait MyAny {
    type Wrapper: ?Sized;

    fn get_wrapper(&self) -> &Self::Wrapper;
}

// impl<T: Any> MyAny for T {}

impl MyAny for Mutex<dyn Any> {
    type Wrapper = Mutex<dyn Any>;
    fn get_wrapper(&self) -> &Self::Wrapper {
        self
    }
}

impl dyn MyAny<Wrapper = Mutex<dyn Any>> {
    fn down_cast<T>(&self) -> MutexGuard<T> {
        let x = self.get_wrapper().lock().unwrap();

        todo!()
    }
}

struct ResStore {
    map: UnsafeCell<HashMap<TypeId, Box<dyn MyAny<Wrapper = Mutex<dyn Any>>>>>,
}

impl ResStore {
    fn insert<T: Any + MyAny + Send + Sync>(&mut self, item: T) -> bool
    where
        std::sync::Mutex<T>: MyAny,
    {
        self.map
            .get_mut()
            .insert(TypeId::of::<T>(), Box::new(Mutex::new(item)))
            .is_some()
    }

    fn get<'a, T: Any>(&'a self) -> Option<&'a T> {
        let ptr: &'a HashMap<TypeId, Box<dyn Any>> = unsafe { self.map.get().as_ref().unwrap() };
        ptr.get(&TypeId::of::<T>()).and_then(|x| x.downcast_ref())
    }
}
