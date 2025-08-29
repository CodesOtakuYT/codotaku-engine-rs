use crate::core::gpu::Gpu;
use std::sync::Arc;
use vulkano::command_buffer::{ClearColorImageInfo, PrimaryAutoCommandBuffer};
use vulkano::format::ClearColorValue;
use vulkano::image::Image;

pub struct RenderParams {
    pub clear_color: [f32; 4],
}

pub struct Renderer {
    gpu: Arc<Gpu>,
}

impl Renderer {
    pub fn new(gpu: Arc<Gpu>) -> Self {
        Self { gpu }
    }

    pub fn render(
        &self,
        image: Arc<Image>,
        render_params: RenderParams,
    ) -> anyhow::Result<Arc<PrimaryAutoCommandBuffer>> {
        let mut builder = self.gpu.create_command_buffer_builder()?;
        builder.clear_color_image(ClearColorImageInfo {
            clear_value: ClearColorValue::Float(render_params.clear_color),
            ..ClearColorImageInfo::image(image)
        })?;
        let command_buffer = builder.build()?;
        Ok(command_buffer)
    }
}
