use crate::core::gpu::Gpu;
use std::sync::Arc;
use vulkano::buffer::{BufferContents, BufferUsage, Subbuffer};
use vulkano::command_buffer::{PrimaryAutoCommandBuffer, RenderingAttachmentInfo, RenderingInfo};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::{PolygonMode, RasterizationState};
use vulkano::pipeline::graphics::subpass::PipelineRenderingCreateInfo;
use vulkano::pipeline::graphics::vertex_input::{Vertex as VertexTrait, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
};
use vulkano::render_pass::{AttachmentLoadOp, AttachmentStoreOp};
use vulkano::shader::EntryPoint;

pub struct RenderParams<Vertex> {
    pub clear_color: [f32; 4],
    pub mesh: Option<Mesh<Vertex>>,
}

pub struct Renderer {
    pipeline: Arc<GraphicsPipeline>,
    gpu: Arc<Gpu>,
}

#[derive(Clone)]
pub struct Mesh<Vertex> {
    vertex_buffer: Subbuffer<[Vertex]>,
    index_buffer: Subbuffer<[u16]>,
}

impl<Vertex: BufferContents> Mesh<Vertex> {
    pub fn new(gpu: Arc<Gpu>, vertices: Vec<Vertex>, indices: Vec<u16>) -> anyhow::Result<Self> {
        let vertex_buffer = gpu.create_buffer(vertices, BufferUsage::VERTEX_BUFFER)?;
        let index_buffer = gpu.create_buffer(indices, BufferUsage::INDEX_BUFFER)?;
        Ok(Self {
            vertex_buffer,
            index_buffer,
        })
    }
}

impl Renderer {
    pub fn new<Vertex: VertexTrait>(
        gpu: Arc<Gpu>,
        image_format: Format,
        vs: EntryPoint,
        fs: EntryPoint,
    ) -> anyhow::Result<Self> {
        let pipeline = {
            let vertex_input_state = Vertex::per_vertex().definition(&vs)?;

            let stages = [
                PipelineShaderStageCreateInfo::new(vs),
                PipelineShaderStageCreateInfo::new(fs),
            ];

            let layout = PipelineLayout::new(
                gpu.queue.device().clone(),
                PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                    .into_pipeline_layout_create_info(gpu.queue.device().clone())
                    .unwrap(),
            )?;

            let subpass = PipelineRenderingCreateInfo {
                color_attachment_formats: vec![Some(image_format)],
                ..Default::default()
            };

            GraphicsPipeline::new(
                gpu.queue.device().clone(),
                None,
                GraphicsPipelineCreateInfo {
                    stages: stages.into_iter().collect(),
                    vertex_input_state: Some(vertex_input_state),
                    input_assembly_state: Some(InputAssemblyState::default()),
                    viewport_state: Some(ViewportState::default()),
                    rasterization_state: Some(RasterizationState {
                        polygon_mode: PolygonMode::Fill,
                        ..Default::default()
                    }),
                    multisample_state: Some(MultisampleState::default()),
                    color_blend_state: Some(ColorBlendState::with_attachment_states(
                        subpass.color_attachment_formats.len() as u32,
                        ColorBlendAttachmentState::default(),
                    )),
                    dynamic_state: [DynamicState::Viewport].into_iter().collect(),
                    subpass: Some(subpass.into()),
                    ..GraphicsPipelineCreateInfo::layout(layout)
                },
            )?
        };

        Ok(Self { pipeline, gpu })
    }

    pub fn render<Vertex>(
        &self,
        image_view: Arc<ImageView>,
        render_params: RenderParams<Vertex>,
    ) -> anyhow::Result<Arc<PrimaryAutoCommandBuffer>> {
        let extent = image_view.image().extent();
        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: [extent[0] as f32, extent[1] as f32],
            depth_range: 0.0..=1.0,
        };
        let mut builder = self.gpu.create_command_buffer_builder()?;
        builder
            .begin_rendering(RenderingInfo {
                color_attachments: vec![Some(RenderingAttachmentInfo {
                    load_op: AttachmentLoadOp::Clear,
                    store_op: AttachmentStoreOp::Store,
                    clear_value: Some(render_params.clear_color.into()),
                    ..RenderingAttachmentInfo::image_view(image_view)
                })],
                ..Default::default()
            })?
            .set_viewport(0, [viewport].into_iter().collect())?
            .bind_pipeline_graphics(self.pipeline.clone())?;
        if let Some(mesh) = render_params.mesh {
            let index_count = mesh.index_buffer.len();
            builder.bind_vertex_buffers(0, mesh.vertex_buffer)?;
            builder.bind_index_buffer(mesh.index_buffer)?;
            unsafe { builder.draw_indexed(index_count as u32, 1, 0, 0, 0) }?;
        }
        builder.end_rendering()?;
        let command_buffer = builder.build()?;
        Ok(command_buffer)
    }
}
