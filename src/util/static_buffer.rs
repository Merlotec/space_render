use std::marker::PhantomData;
use std::mem;
use std::ops::Range;

use amethyst::renderer::{
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
        resource::{
            Buffer,
            BufferInfo,
            Escape,
        },
    },
    types::Backend,
};

#[derive(Debug)]
/// A helper struct use to simplify the use of a read only vertex buffer which is constant.
pub struct StaticVertexBuffer<B: Backend, T> {
    vertex_buffer: Escape<Buffer<B>>,
    vertex_count: usize,

    index_buffer: Option<(Escape<Buffer<B>>, usize)>,

    phantom: PhantomData<T>,
}

impl<B: Backend, T> StaticVertexBuffer<B, T> {
    pub fn allocate(factory: &Factory<B>, queue: QueueId, vertex_data: &[T], index_data: Option<&[u32]>) -> Result<Self, failure::Error> {
        let index_buffer: Option<(Escape<Buffer<B>>, usize)> = {
            if let Some(index_data) = index_data {
                Some(
                    (
                        alloc_read_optimal(
                            factory,
                            queue,
                            hal::buffer::Usage::INDEX,
                            hal::pso::PipelineStage::VERTEX_INPUT,
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
                vertex_buffer: alloc_read_optimal(
                    factory,
                    queue,
                    hal::buffer::Usage::VERTEX,
                    hal::pso::PipelineStage::VERTEX_INPUT,
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
    let mut staging = factory
        .create_buffer(
            BufferInfo {
                size: (data.len() * mem::size_of::<T>()) as u64,
                usage: hal::buffer::Usage::TRANSFER_SRC,
            },
            memory::Dynamic,
        )?;

    let buffer = factory
        .create_buffer(
            BufferInfo {
                size: (data.len() * mem::size_of::<T>()) as u64,
                usage: hal::buffer::Usage::TRANSFER_DST | usage,
            },
            memory::Data,
        )?;

    unsafe {
        factory
            .upload_visible_buffer(
                &mut staging,
                0,
                data,
            )?;
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