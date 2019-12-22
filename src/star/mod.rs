pub mod sub;
pub mod pass;
use amethyst::{
    assets::PrefabData,
    derive::PrefabData,
    core::math::{
        Vector3,
    },
    renderer::palette::Srgb,
    error::Error,
    ecs::prelude::*,
};

use serde::{Serialize, Deserialize};

use glsl_layout::*;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PrefabData)]
#[prefab(Component)]
pub struct Star {
    #[serde(with = "amethyst::renderer::serde_shim::srgb")]
    pub color: Srgb,
}

impl Star {
    pub fn new(color: Srgb) -> Self {
        Self { color }
    }
}

impl Default for Star {
    fn default() -> Self {
        Self::new(Srgb::new(1.0, 1.0, 1.0))
    }
}

impl Component for Star {
    type Storage = DenseVecStorage<Self>;
}

pub const MAX_STARS: usize = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, AsStd140)]
#[repr(C, align(4))]
pub(crate) struct StarData {
    pub center: vec3,
    pub radius: float,
    pub color: vec3,
}

impl StarData {
    pub(crate) fn new(star: &Star, center: Vector3<f32>, radius: f32) -> Self {
        Self {
            center: Into::<[f32; 3]>::into(center).into(),
            radius,
            color: [star.color.red, star.color.green, star.color.blue].into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, AsStd140)]
#[repr(C, align(4))]
pub(crate) struct StarList {
    count: uint,
    stars: [StarData; MAX_STARS],
}

impl StarList {
    pub(crate) fn new(star_data: &[StarData]) -> Self {
        assert!(star_data.len() <= MAX_STARS);
        let mut stars: [StarData; MAX_STARS] = Default::default();
        for (i, data) in star_data.iter().enumerate() {
            stars[i] = *data;
        }
        Self { stars, count: star_data.len() as u32 }
    }
}