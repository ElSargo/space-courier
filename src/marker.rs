use bevy_ecs::prelude::{Component, Resource};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

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
