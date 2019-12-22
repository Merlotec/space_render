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
pub struct PerImageDynamicShaderBuffer<B: Backend, T> {
    buffer: Escape<Buffer<B>>,
    set: Escape<DescriptorSet<B>>,
    count: usize,
    marker: PhantomData<T>,
}

#[derive(Debug)]
pub struct DynamicShaderBuffer<B: Backend, T> {
    layout: Handle<DescriptorSetLayout<B>>,
    per_image: Vec<PerImageDynamicShaderBuffer<B, T>>,
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

    #[inline]
    pub fn buffer_len(&self, index: usize) -> usize {
        if let Some(this_image) = self.per_image.get(index) {
            this_image.count
        } else {
            0
        }
    }

    pub fn write(&mut self, factory: &Factory<B>, index: usize, data: &[T]) -> bool {
        let mut changed = false;
        let this_image = {
            while self.per_image.len() <= index {
                self.per_image
                    .push(PerImageDynamicShaderBuffer::new(factory, &self.layout, mem::size_of::<T>() * data.len(), data.len()));
                changed = true;
            }
            &mut self.per_image[index]
        };

        {
            let mut mapped = this_image.map(factory);
            let mut writer = unsafe {
                mapped
                    .write::<u8>(factory.device(), 0..(std::mem::size_of::<T>() * data.len()) as u64)
                    .unwrap()
            };
            let slice = unsafe { writer.slice() };
            
            slice.copy_from_slice(util::slice_as_bytes(data));
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

    pub fn read(&mut self, factory: &Factory<B>, index: usize) -> Option<Vec<T>> where T: std::marker::Copy {
        if let Some(this_image) = self.per_image.get_mut(index) {
            let count = this_image.count;
            let mut mapped: MappedRange<B> = this_image.map(factory);
            let bytes = unsafe {
                mapped.read::<T>(factory, 0..(count * mem::size_of::<T>()) as u64)
            }.ok()?;
            let vec: Vec<T> = bytes.to_vec();
            Some(vec)
        } else {
            None
        }
    }
}

impl<B: Backend, T: AsStd140> DynamicShaderBuffer<B, T>
    where
        T::Std140: Sized,
{
    pub fn write_formatted(&mut self, factory: &Factory<B>, index: usize, data: &[T]) -> bool {
        let mut formatted = Vec::with_capacity(data.len());
        for item in data {
            formatted.push(item.std140());
        }
        let mut changed = false;
        let this_image = {
            while self.per_image.len() <= index {
                self.per_image.push(PerImageDynamicShaderBuffer::new(factory, &self.layout, mem::size_of::<T::Std140>() * formatted.len(), formatted.len()));
                changed = true;
            }
            &mut self.per_image[index]
        };
        {
            let mut mapped = this_image.map(factory);
            let mut writer = unsafe {
                mapped
                    .write::<u8>(factory.device(), 0..(mem::size_of::<T::Std140>() * formatted.len()) as u64)
                    .unwrap()
            };
            let slice = unsafe { writer.slice() };
        
            slice.copy_from_slice(util::slice_as_bytes(formatted.as_slice()));
        }
        this_image.count = formatted.len();
        changed
    }
}

impl<B: Backend, T> PerImageDynamicShaderBuffer<B, T> {
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