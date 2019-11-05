use std::marker::PhantomData;
use std::mem;
use std::ops::Range;

use amethyst::renderer::{
    rendy::{
        command::RenderPassEncoder,
        factory::Factory,
        hal,
        memory,
        resource::{
            Buffer,
            BufferInfo,
            Escape,
        },
    },
    types::Backend,
};

#[derive(Debug)]
pub struct StaticVertexBuffer<B: Backend, T> {
    vertex_buffer: Escape<Buffer<B>>,
    vertex_count: usize,

    index_buffer: Option<(Escape<Buffer<B>>, usize)>,

    phantom: PhantomData<T>,
}

impl<B: Backend, T> StaticVertexBuffer<B, T> {
    pub fn allocate(factory: &Factory<B>, vertex_data: &[T], index_data: Option<&[u32]>) -> Result<Self, failure::Error> {
        let index_buffer: Option<(Escape<Buffer<B>>, usize)> = {
            if let Some(index_data) = index_data {
                Some((alloc_simple(factory, hal::buffer::Usage::INDEX, memory::Dynamic, index_data)?, index_data.len()))
            } else {
                None
            }
        };
        Ok(
            Self {
                vertex_buffer: alloc_simple(factory, hal::buffer::Usage::VERTEX, memory::Dynamic, vertex_data)?,
                vertex_count: vertex_data.len(),
                index_buffer,
                phantom: PhantomData,
            }
        )
    }

    pub unsafe fn draw(&self, encoder: &mut RenderPassEncoder<'_, B>, instances: Range<u32>) {
        encoder.bind_vertex_buffers(0, Some((self.vertex_buffer.raw(), 0)));
        if let Some((index_buffer, count)) = self.index_buffer.as_ref() {
            // Draw indexed.
            encoder.bind_index_buffer(index_buffer.raw(), 0, hal::IndexType::U32);
            encoder.draw_indexed(0..(*count) as u32, 0, instances);
        } else {
            encoder.draw(0..self.vertex_count as u32, instances);
        }
    }
}

fn alloc_simple<B: hal::Backend, T>(factory: &Factory<B>, usage: hal::buffer::Usage, memory_usage: impl memory::MemoryUsage, data: &[T]) -> Result<Escape<Buffer<B>>, failure::Error> {
    let mut buffer = factory
        .create_buffer(
            BufferInfo {
                size: (data.len() * mem::size_of::<T>()) as u64,
                usage,
            },
            memory_usage,
        )?;

    unsafe {
        factory
            .upload_visible_buffer(
                &mut buffer,
                0,
                data,
            )?;
    }
    Ok(buffer)
}