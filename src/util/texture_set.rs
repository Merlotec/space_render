use std::marker::PhantomData;
use std::mem;

use amethyst::renderer::{
    rendy::{
        command::RenderPassEncoder,
        factory::Factory,
        command::QueueId,
        hal,
        hal::Device,
        memory,
        memory::{MappedRange, Write},
        resource::{
            Buffer,
            BufferInfo,
            DescriptorSet,
            DescriptorSetLayout,
            Escape,
            Handle
        },
        texture::Texture,
    },
    types::Backend,
    util,
};
use glsl_layout::AsStd140;

#[derive(Debug)]
pub struct PerImageTextureSet<B: Backend> {
    set: Escape<DescriptorSet<B>>,
    texture: Texture<B>,
}

#[derive(Debug)]
pub struct TextureSet<B: Backend> {
    layout: Handle<DescriptorSetLayout<B>>,
    per_image: Vec<PerImageTextureSet<B>>,
}

impl<B: Backend> TextureSet<B> {
    pub fn new(factory: &Factory<B>, flags: hal::pso::ShaderStageFlags) -> Result<Self, failure::Error> {
        Ok(Self {
            layout: factory
                .create_descriptor_set_layout(util::set_layout_bindings(Some((
                    1,
                    hal::pso::DescriptorType::CombinedImageSampler,
                    flags,
                ))))?
                .into(),
            per_image: Vec::new(),
        })
    }

    /// Returns the `DescriptSetLayout` for this set.
    #[inline]
    pub fn raw_layout(&self) -> &B::DescriptorSetLayout {
        self.layout.raw()
    }

    pub fn invalidate(&mut self) {
        self.per_image.clear();
    }

    #[inline]
    pub fn contains_image_at(&self, index: usize) -> bool {
        self.per_image.len() > index
    }


    pub fn write_unique(&mut self, factory: &Factory<B>, index: usize, texture: Texture<B>) -> bool {
        let mut changed = false;
        if (index >= self.per_image.len()) {
            if (index == self.per_image.len()) {
                self.per_image.push(PerImageTextureSet::new(factory, &self.layout, texture));
                changed = true;
            } else {
                panic!("Tried to write to an index greater than the len + 1.");
            }
        } else {
            self.per_image[index] = PerImageTextureSet::new(factory, &self.layout, texture);
        }
        changed
    }

    /// Bind this descriptor set
    #[inline]
    pub fn bind(
        &self,
        index: usize,
        pipeline_layout: &B::PipelineLayout,
        binding_id: u32,
        encoder: &mut RenderPassEncoder<'_, B>,
    ) {
        self.per_image[index].bind(pipeline_layout, binding_id, encoder);
    }
}

impl<B: Backend> PerImageTextureSet<B> {
    fn new(factory: &Factory<B>, layout: &Handle<DescriptorSetLayout<B>>, texture: Texture<B>) -> Self {
        let set = factory.create_descriptor_set(layout.clone()).unwrap();
        let desc = hal::pso::Descriptor::CombinedImageSampler(texture.view().raw(), hal::image::Layout::ShaderReadOnlyOptimal, texture.sampler().raw());
        unsafe {
            let set = set.raw();
            factory.write_descriptor_sets(Some(util::desc_write(set, 0, desc)));
        }
        Self {
            set,
            texture,
        }
    }

    #[inline]
    fn bind(
        &self,
        pipeline_layout: &B::PipelineLayout,
        set_id: u32,
        encoder: &mut RenderPassEncoder<'_, B>,
    ) {
        unsafe {
            encoder.bind_graphics_descriptor_sets(
                pipeline_layout,
                set_id,
                Some(self.set.raw()),
                std::iter::empty(),
            );
        }
    }
}