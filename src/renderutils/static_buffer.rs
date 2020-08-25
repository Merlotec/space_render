use std::marker::PhantomData;
use std::mem;
use std::ops::Range;

use amethyst::{
    renderer::{
        rendy::{
            command::{
                QueueId,
                RenderPassEncoder,
            },
            factory::{
                Factory,
                BufferState,
            },
            hal,
            memory,
            memory::{
                Write,
            },
            resource::{
                Buffer,
                BufferInfo,
                Escape,
            },
        },
        types::Backend,
        util,
    },
};
use std::alloc::alloc;

#[derive(Debug)]
/// A helper struct use to simplify the use of a read only vertex buffer which is constant.
pub struct StaticVertexBuffer<B: Backend, T> {
    per_image: Vec<PerImageStaticVertexBuffer<B, T>>,
}

impl<B: Backend, T> StaticVertexBuffer<B, T> {
    pub fn new() -> Self {
        Self {
            per_image: vec![],
        }
    }

    pub fn prepare(&mut self, factory: &Factory<B>, queue: QueueId, vertex_data: &[T], index_data: Option<&[u32]>, index: usize) -> Result<(), failure::Error> {
       if self.per_image.len() <= index {
           self.per_image.insert(
               index,
               PerImageStaticVertexBuffer::allocate(
                   factory,
                   queue,
                   vertex_data,
                   index_data,
               )?
           );
       }
        Ok(())
    }

    pub unsafe fn draw(&self, encoder: &mut RenderPassEncoder<'_, B>, instances: Range<u32>, index: usize) {
        self.per_image[index].draw(encoder, instances);
    }
}

#[derive(Debug)]
pub struct PerImageStaticVertexBuffer<B: Backend, T> {
    vertex_buffer: Escape<Buffer<B>>,
    vertex_count: usize,
    index_buffer: Option<(Escape<Buffer<B>>, usize)>,
    phantom: PhantomData<T>,
}

impl<B: Backend, T> PerImageStaticVertexBuffer<B, T> {
    pub fn allocate(factory: &Factory<B>, queue: QueueId, vertex_data: &[T], index_data: Option<&[u32]>) -> Result<Self, failure::Error> {
        let index_buffer: Option<(Escape<Buffer<B>>, usize)> = {
            if let Some(index_data) = index_data {
                Some(
                    (
                        // We need to use alloc_dynamic because the other one causes strange instability...
                        alloc_dynamic(
                            factory,
                            hal::buffer::Usage::INDEX,
                            index_data
                        )?,
                        index_data.len()
                    )
                )
            } else {
                None
            }
        };
        Ok(
            Self {
                // We need to use alloc_dynamic because the other one causes strange instability...
                vertex_buffer: alloc_dynamic(
                    factory,
                    hal::buffer::Usage::VERTEX,
                    vertex_data
                )?,
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

/// Creates a GPU read optimal buffer which is optimised for the fastest possible read speeds.
/// This means that uploading data to the buffer is more laborious, but since we aren't changing the data it doesn't matter.
fn alloc_read_optimal<B: hal::Backend, T>(factory: &Factory<B>, queue: QueueId, usage: hal::buffer::Usage, stage: hal::pso::PipelineStage, data: &[T]) -> Result<Escape<Buffer<B>>, failure::Error> {
    let buffer_size = mem::size_of_val(data) as u64;

    let mut staging = factory
        .create_buffer(
            BufferInfo {
                size: buffer_size,
                usage: hal::buffer::Usage::TRANSFER_SRC,
            },
            memory::Upload,
        )?;

    let mut buffer = factory
        .create_buffer(
            BufferInfo {
                size: buffer_size,
                usage: hal::buffer::Usage::TRANSFER_DST | usage,
            },
            memory::Data,
        )?;

    unsafe {
        {
            let mut mapped = staging.map(factory.device(), 0..buffer_size)?;
            let mut writer = mapped.write::<u8>(factory.device(), 0..buffer_size)?;
            let slice = writer.slice();
            let data_slice = util::slice_as_bytes(data);
            slice.copy_from_slice(data_slice);
        }

        factory.upload_from_staging_buffer(
            &buffer,
            0,
            staging,
            None,
            BufferState::new(queue)
                .with_stage(stage)

        )?;
    }
    Ok(buffer)
}

fn alloc_dynamic<B: hal::Backend, T>(factory: &Factory<B>, usage: hal::buffer::Usage, data: &[T]) -> Result<Escape<Buffer<B>>, failure::Error> {
    let buffer_size = mem::size_of_val(data) as u64;

    let mut buffer = factory
        .create_buffer(
            BufferInfo {
                size: buffer_size,
                usage,
            },
            memory::Dynamic,
        )?;

    unsafe {
        let mut mapped = buffer.map(factory.device(), 0..buffer_size)?;
        let mut writer = mapped.write::<u8>(factory.device(), 0..buffer_size)?;
        let slice = writer.slice();
        let data_slice = util::slice_as_bytes(data);
        slice.copy_from_slice(data_slice);
    }
    Ok(buffer)
}