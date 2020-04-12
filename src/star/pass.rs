use std::ops::Range;

use amethyst::{
    assets::{
        Loader,
        AssetStorage,
    },
    core::ecs::{
        DispatcherBuilder, World,
    },
    error::Error,
    renderer::{
        bundle::{RenderOrder, RenderPlan, RenderPlugin, Target},
        pipeline::{PipelineDescBuilder, PipelinesBuilder},
        rendy::{
            command::{QueueId, RenderPassEncoder},
            factory::Factory,
            graph::{
                GraphContext,
                NodeBuffer, NodeImage, render::{PrepareResult, RenderGroup, RenderGroupDesc},
            },
            hal::{self, device::Device,  pso, pso::ShaderStageFlags},
            mesh::{AsVertex, Position, TexCoord, PosTex},
            shader::{Shader, SpirvShader},
        },
        submodules::{
            FlatEnvironmentSub,
            TextureSub,
        },
        Texture,
        types::Backend, util,
    },
};

use super::*;
use crate::{
    star::sub::*,
};

use crate::renderutils::*;

use amethyst::prelude::WorldExt;

const STATIC_DEPTH: f32 = 0.0;
const STATIC_CROP: f32 = 0.2;

const STATIC_VERTEX_DATA: [PosTex; 4] = [
    PosTex { position: Position([-1.0, -1.0, STATIC_DEPTH]), tex_coord: TexCoord([0.0 + STATIC_CROP, 0.0 + STATIC_CROP]) },
    PosTex { position: Position([-1.0, 1.0, STATIC_DEPTH]), tex_coord: TexCoord([0.0 + STATIC_CROP, 1.0 - STATIC_CROP]) },
    PosTex { position: Position([1.0, 1.0, STATIC_DEPTH]), tex_coord: TexCoord([1.0 - STATIC_CROP, 1.0 - STATIC_CROP]) },
    PosTex { position: Position([1.0, -1.0, STATIC_DEPTH]), tex_coord: TexCoord([1.0 - STATIC_CROP, 0.0 + STATIC_CROP]) },
];

const STATIC_INSTANCE_DATA: [u32; 6] = [0, 1, 2, 0, 3, 2];


lazy_static::lazy_static! {
    static ref VERTEX: SpirvShader = SpirvShader::from_bytes(
        include_bytes!("../../shaders/spirv/star.vert.spv"),
        ShaderStageFlags::VERTEX,
        "main",
    ).unwrap();

    static ref FRAGMENT: SpirvShader = SpirvShader::from_bytes(
        include_bytes!("../../shaders/spirv/star.frag.spv"),
        ShaderStageFlags::FRAGMENT,
        "main",
    ).unwrap();
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawStarDesc;

impl DrawStarDesc {
    /// Create instance of `DrawStarDesc` render group
    pub fn new() -> Self {
        Default::default()
    }
}

impl<B: Backend> RenderGroupDesc<B, World> for DrawStarDesc {
    fn build(
        self,
        _ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        queue: QueueId,
        _world: &World,
        framebuffer_width: u32,
        framebuffer_height: u32,
        subpass: hal::pass::Subpass<'_, B>,
        _buffers: Vec<NodeBuffer>,
        _images: Vec<NodeImage>,
    ) -> Result<Box<dyn RenderGroup<B, World>>, failure::Error> {
        let env = FlatEnvironmentSub::new(factory)?;
        let stars = StarSub::new(
            factory,
            hal::pso::ShaderStageFlags::VERTEX | hal::pso::ShaderStageFlags::FRAGMENT
        )?;
        let tex = TextureSub::new(factory)?;
        
        // Load billboard mesh.
        let vertex = StaticVertexBuffer::allocate(factory, queue, &STATIC_VERTEX_DATA, Some(&STATIC_INSTANCE_DATA))?;
        let (pipeline, pipeline_layout) = build_custom_pipeline(
            factory,
            subpass,
            framebuffer_width,
            framebuffer_height,
            vec![env.raw_layout(), stars.raw_layout(), tex.raw_layout()],
            None,
        )?;

        // let query_pool = factory.device().create_query_pool(
        //     query::Type::Occlusion,
        //     1,
        // );

        Ok(Box::new(DrawStar::<B> {
            pipeline,
            pipeline_layout,
            env,
            vertex,
            stars,
            tex,
            // query_pool,
        }))
    }
}


#[derive(Debug)]
pub struct DrawStar<B: Backend> {
    pipeline: B::GraphicsPipeline,
    pipeline_layout: B::PipelineLayout,
    env: FlatEnvironmentSub<B>,
    vertex: StaticVertexBuffer<B, PosTex>,
    stars: StarSub<B>,
    tex: TextureSub<B>,
    // query_pool: B::QueryPool,
}

impl<B: Backend> RenderGroup<B, World> for DrawStar<B> {
    fn prepare(
        &mut self,
        factory: &Factory<B>,
        _queue: QueueId,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        world: &World,
    ) -> PrepareResult {

        self.env.process(factory, index, world);
        self.stars.process(factory, index, world);

        // Load any unloaded textures.
        // TODO: make more efficient!
        if let Some(mut star_texture) = world.try_fetch_mut::<StarTexture>() {
            if let Some((texture, _)) = self.tex.insert(factory, world, &star_texture.texture, hal::image::Layout::ShaderReadOnlyOptimal) {
                star_texture.tex_id = Some(texture);
            }
        }
        self.tex.maintain(factory, world);

        PrepareResult::DrawRecord
    }

    fn draw_inline(
        &mut self,
        mut encoder: RenderPassEncoder<'_, B>,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        world: &World,
    ) {
        if !self.stars.is_empty() {
            encoder.bind_graphics_pipeline(&self.pipeline);
            self.env.bind(index, &self.pipeline_layout, 0, &mut encoder);
            self.stars.bind(index, &self.pipeline_layout, 1, &mut encoder);
            if let Some(star_texture) = world.try_fetch::<StarTexture>() {
                if let Some(texture_id) = star_texture.tex_id {
                    if self.tex.loaded(texture_id) {
                        self.tex.bind(&self.pipeline_layout, 2, texture_id, &mut encoder);
                        unsafe {
                            self.vertex.draw(&mut encoder, 0..self.stars.count() as u32);
                        }
                    }
                }
            }
        }
    }

    fn dispose(self: Box<Self>, factory: &mut Factory<B>, _world: &World) {
        unsafe {
            factory.device().destroy_graphics_pipeline(self.pipeline);
            factory
                .device()
                .destroy_pipeline_layout(self.pipeline_layout);
        }
    }
}

fn build_custom_pipeline<B: Backend>(
    factory: &Factory<B>,
    subpass: hal::pass::Subpass<'_, B>,
    framebuffer_width: u32,
    framebuffer_height: u32,
    layouts: Vec<&B::DescriptorSetLayout>,
    push_constant: Option<(hal::pso::ShaderStageFlags, Range<u32>)>,
) -> Result<(B::GraphicsPipeline, B::PipelineLayout), failure::Error> {
    let pipeline_layout = unsafe {
        factory
            .device()
            .create_pipeline_layout(layouts, push_constant)
    }?;
    // Load the shaders
    let shader_vertex = unsafe { VERTEX.module(factory).unwrap() };
    let shader_fragment = unsafe { FRAGMENT.module(factory).unwrap() };

    // Build the pipeline
    let pipes = PipelinesBuilder::new()
        .with_pipeline(
            PipelineDescBuilder::new()
                .with_vertex_desc(&[(PosTex::vertex(), pso::VertexInputRate::Vertex)])
                .with_input_assembler(pso::InputAssemblerDesc::new(hal::Primitive::TriangleList))
                .with_shaders(util::simple_shader_set(
                    &shader_vertex,
                    Some(&shader_fragment),
                ))
                .with_layout(&pipeline_layout)
                .with_subpass(subpass)
                .with_framebuffer_size(framebuffer_width, framebuffer_height)
                .with_depth_test(pso::DepthTest {
                    fun: pso::Comparison::Less,
                    write: true,
                })
                .with_blend_targets(vec![pso::ColorBlendDesc { blend: Some(pso::BlendState::ALPHA), mask: pso::ColorMask::ALL}]),
        )
        .build(factory, None);
    unsafe {
        factory.destroy_shader_module(shader_vertex);
        factory.destroy_shader_module(shader_fragment);
    }
    match pipes {
        Err(e) => {
            unsafe {
                factory.device().destroy_pipeline_layout(pipeline_layout);
            }
            Err(e)
        }
        Ok(mut pipes) => Ok((pipes.remove(0), pipeline_layout)),
    }
}
#[derive(Debug, Default)]
pub struct StarRender {
    flash_path: String,
}

impl StarRender {
    pub fn new(flash_path: impl Into<String>) -> Self {
        Self {
            flash_path: flash_path.into(),
        }
    }
}


impl<B: Backend> RenderPlugin<B> for StarRender {
    fn on_build<'a, 'b>(
        &mut self,
        world: &mut World,
        _builder: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        let tex = {
            if !world.has_value::<AssetStorage::<Texture>>() {
                world.insert(AssetStorage::<Texture>::new());
            }
            let loader = world.read_resource::<Loader>();
            loader.load(
                &self.flash_path,
                amethyst::renderer::formats::texture::ImageFormat::default(),
                (),
                &world.read_resource::<AssetStorage<Texture>>(),
            )
        };

        world.insert(StarTexture::new(tex));
        world.register::<crate::Star>();
        Ok(())
    }

    fn on_plan(
        &mut self,
        plan: &mut RenderPlan<B>,
        _factory: &mut Factory<B>,
        _world: &World,
    ) -> Result<(), Error> {
        plan.extend_target(Target::Main, |ctx| {
            ctx.add(RenderOrder::DisplayPostEffects, DrawStarDesc::new().builder())?;
            Ok(())
        });
        Ok(())
    }
}