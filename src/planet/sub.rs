use amethyst::{
    core::{
        transform::Transform,
        math::{
            Matrix4,
            Vector4,
        }
    },
    renderer::{
        submodules::DynamicUniform,
        rendy::{
            command::RenderPassEncoder,
            factory::Factory,
            hal,
        },
        types::Backend,
    },
    ecs::prelude::*,
};

use super::*;

#[derive(Debug)]
pub(crate) struct PlanetSub<B: Backend> {
    uniform: DynamicUniform<B, PlanetList>,
    data: PlanetList,
}

impl<B: Backend> PlanetSub<B> {
    pub fn new(factory: &Factory<B>, flags: hal::pso::ShaderStageFlags) -> Result<Self, failure::Error> {
        let uniform = DynamicUniform::new(factory, flags)?;
        Ok(Self { uniform, data: PlanetList::default() })
    }

    pub fn process(&mut self, factory: &Factory<B>, index: usize, world: &World) {
        let mut planet_list: Vec<PlanetData> = Vec::new();
        for (atmosphere, transform) in (&world.read_storage::<Atmosphere>(), &world.read_storage::<Transform>()).join() {
            let matrix: Matrix4<f32> = *transform.global_matrix();
            let translation: Vector4<f32> = matrix.column(3).into();
            if matrix.column(0)[0].abs() == matrix.column(1)[1].abs() && matrix.column(1)[1].abs() == matrix.column(2)[2].abs() {
                // The scale is uniform - this is good.
                planet_list.push(PlanetData::new(&atmosphere,translation.xyz(), matrix.row(0)[0].abs()));
            } else {
                // The scale is non uniform, which means that we cannot extract a radius for the planet.
                panic!("Non uniform scale provided for planet! We need a uniform scale (x, y, z components of scale are the same) to determine the radius of the planet, as it is spherical.");
            }
        }
        self.data = PlanetList::new(planet_list.as_slice());
        self.uniform.write(factory, index, self.data.std140());
    }

    /// Returns the raw `DescriptorSetLayout` for this environment
    pub fn raw_layout(&self) -> &B::DescriptorSetLayout {
        self.uniform.raw_layout()
    }

    pub fn bind(&mut self, index: usize, pipeline_layout: &B::PipelineLayout, binding_id: u32, encoder: &mut RenderPassEncoder<B>) {
        self.uniform.bind(index,  pipeline_layout, binding_id, encoder);
    }

    pub fn is_empty(&self) -> bool {
        self.data.count == 0
    }
}

