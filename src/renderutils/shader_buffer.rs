use std::marker::PhantomData;
use std::mem;

use amethyst::renderer::{
    rendy::{
        command::RenderPassEncoder,
        factory::Factory,
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
    },
    types::Backend,
    util,
};
use glsl_layout::AsStd140;

#[derive(Debug)]
pub struct DynamicShaderBufferBinding<B: Backend, T> {
    buffer: Escape<Buffer<B>>,
    set: Escape<DescriptorSet<B>>,
    count: usize,
    marker: PhantomData<T>,
}

/// Represents a dynamically sized shader buffer that may grow at the discretion of the user.
/// This is slighlty slower than uniform buffers due to the nature of the memory and access.
/// Whenever possible, a uniform buffer should be used, but in cases where large amounts of memory may be saved depending on circumstance, this is an option.
#[derive(Debug)]
pub struct DynamicShaderBuffer<B: Backend, T> {
    layout: Handle<DescriptorSetLayout<B>>,
    binding_data: Option<DynamicShaderBufferBinding<B, T>>,
}

impl<B: Backend, T> DynamicShaderBuffer<B, T> {
    pub fn new(factory: &Factory<B>, flags: hal::pso::ShaderStageFlags) -> Result<Self, failure::Error> {
        Ok(Self {
            layout: factory
                .create_descriptor_set_layout(util::set_layout_bindings(Some((
                    1,
                    hal::pso::DescriptorType::StorageBuffer,
                    flags,
                ))))?
                .into(),
            binding_data: None,
        })
    }

    /// Returns the `DescriptSetLayout` for this set.
    #[inline]
    pub fn raw_layout(&self) -> &B::DescriptorSetLayout {
        self.layout.raw()
    }

    /// Results in each image being reconstructed by invalidating the buffer.
    #[inline]
    pub fn invalidate(&mut self) {
        self.binding_data.take();
    }

    #[inline]
    pub fn has_data(&self) -> bool {
        self.buffer_len() != 0
    }

    #[inline]
    pub fn buffer_len(&self) -> usize {
        if let Some(binding) = self.binding_data.as_ref() {
            binding.count
        } else {
            0
        }
    }

    /// Bind this descriptor set
    #[inline]
    pub fn bind(
        &self,
        pipeline_layout: &B::PipelineLayout,
        binding_id: u32,
        encoder: &mut RenderPassEncoder<'_, B>,
    ) -> Result<(), ()>{
        if let Some(binding_data) = self.binding_data.as_ref() {
            binding_data.bind(pipeline_layout, binding_id, encoder);
            Ok(())
        } else {
            Err(())
        }
    }
}

impl<B: Backend, T: AsStd140> DynamicShaderBuffer<B, T>
    where
        T::Std140: Sized,
{
    pub fn write(&mut self, factory: &Factory<B>, data: &[T]) {
        let mut formatted = Vec::with_capacity(data.len());
        for item in data {
            formatted.push(item.std140());
        }
        self.binding_data = Some(DynamicShaderBufferBinding::new(factory, &self.layout, mem::size_of::<T::Std140>() * formatted.len(), formatted.len()));
        if let Some(binding_data) = self.binding_data.as_mut() {
            let mut mapped = binding_data.map(factory);
            let mut writer = unsafe {
                mapped
                    .write::<u8>(factory.device(), 0..(mem::size_of::<T::Std140>() * formatted.len()) as u64)
                    .unwrap()
            };
            let slice = unsafe { writer.slice() };

            slice.copy_from_slice(util::slice_as_bytes(formatted.as_slice()));
        }
    }
}

impl<B: Backend, T> DynamicShaderBufferBinding<B, T> {
    fn new(factory: &Factory<B>, layout: &Handle<DescriptorSetLayout<B>>, size: usize, count: usize) -> Self {
        let buffer = factory
            .create_buffer(
                BufferInfo {
                    size: size as u64,
                    usage: hal::buffer::Usage::STORAGE,
                },
                memory::Dynamic,
            )
            .unwrap();

        let set = factory.create_descriptor_set(layout.clone()).unwrap();
        let desc = hal::pso::Descriptor::Buffer(buffer.raw(), None..None);
        unsafe {
            let set = set.raw();
            factory.write_descriptor_sets(Some(util::desc_write(set, 0, desc)));
        }
        Self {
            buffer,
            set,
            count,
            marker: PhantomData,
        }
    }

    fn map<'a>(&'a mut self, factory: &Factory<B>) -> MappedRange<'a, B> {
        let range = 0..self.buffer.size();
        self.buffer.map(factory.device(), range).unwrap()
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