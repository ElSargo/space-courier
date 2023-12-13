use std::default::Default;

use glam::{Quat, Vec3};

use crate::transform::{AsTransform, TransformView};

#[derive(Default, Debug)]
pub struct Planet {
    pub translation: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    pub scale: Vec3,
    pub masse: f32,
}

#[derive(Default, Debug)]
pub struct PlanetView {
    pub translation: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    pub scale: Vec3,
    pub masse: f32,
}
impl AsTransform for Planet {
    fn as_transform(&self) -> TransformView {
        TransformView {
            scale: &self.scale,
            translation: &self.translation,
            rotation: &self.rotation,
        }
    }
    fn as_transform_mut(&mut self) -> TransformView {
        TransformView {
            scale: &mut self.scale,
            translation: &mut self.translation,
            rotation: &mut self.rotation,
        }
    }
}
#[derive(Default, Debug)]
pub struct Planets {
    pub translations: Vec<Vec3>,
    pub rotations: Vec<Quat>,
    pub velocity: Vec<Vec3>,
    pub scales: Vec<Vec3>,
    pub masses: Vec<f32>,
}

impl FromIterator<Planet> for Planets {
    fn from_iter<T: IntoIterator<Item = Planet>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let mut res = match iter.size_hint() {
            (_, Some(len)) => Self::with_capacity(len),
            (len, None) => Self::with_capacity(len),
            _ => Self::default(),
        };
        for planet in iter {
            res.push(planet);
        }
        res
    }
}

impl Planets {
    fn with_capacity(size: usize) -> Self {
        Self {
            translations: Vec::with_capacity(size),
            rotations: Vec::with_capacity(size),
            velocity: Vec::with_capacity(size),
            scales: Vec::with_capacity(size),
            masses: Vec::with_capacity(size),
        }
    }

    fn push(&mut self, planet: Planet) {
        self.translations.push(planet.translation);
        self.rotations.push(planet.rotation);
        self.velocity.push(planet.velocity);
        self.scales.push(planet.scale);
        self.masses.push(planet.masse);
    }

    fn get(&mut self, index: usize) -> PlanetView {
        if index < self.translations.len() {}
    }
}

impl IntoIterator for Planets {
    type Item = Planet;

    type IntoIter = PlanetIterator;

    fn into_iter(self) -> Self::IntoIter {
        todo!()
    }
}

struct PlanetIterator<'a> {
    iter: &'a Planets,
    i: usize,
}

impl Iterator for PlanetIterator {
    type Item = Planet;

    fn next(&mut self) -> Option<Self::Item> {}
}
