use crate::core::gpu::Gpu;
use ash::vk;
use ash::vk::ClearColorValue;
use std::sync::Arc;

pub(crate) struct CommandEncoder {
    command_buffer: vk::CommandBuffer,
    gpu: Arc<Gpu>,
}

#[derive(Clone, Copy)]
pub(crate) struct ImageState {
    pub(crate) layout: vk::ImageLayout,
    pub(crate) stage_mask: vk::PipelineStageFlags2,
    pub(crate) access_mask: vk::AccessFlags2,
}

impl ImageState {
    pub(crate) fn none() -> Self {
        ImageState {
            layout: vk::ImageLayout::UNDEFINED,
            stage_mask: vk::PipelineStageFlags2::NONE,
            access_mask: vk::AccessFlags2::NONE,
        }
    }

    pub(crate) fn present() -> Self {
        ImageState {
            layout: vk::ImageLayout::PRESENT_SRC_KHR,
            stage_mask: vk::PipelineStageFlags2::NONE,
            access_mask: vk::AccessFlags2::NONE,
        }
    }

    pub(crate) fn clear() -> Self {
        ImageState {
            layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            stage_mask: vk::PipelineStageFlags2::TRANSFER,
            access_mask: vk::AccessFlags2::TRANSFER_WRITE,
        }
    }
}

impl CommandEncoder {
    pub(crate) fn new(gpu: Arc<Gpu>, command_buffer: vk::CommandBuffer) -> anyhow::Result<Self> {
        unsafe {
            gpu.device
                .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::default())?;
            gpu.device.begin_command_buffer(
                command_buffer,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;
        }
        Ok(Self {
            command_buffer,
            gpu,
        })
    }

    pub fn clear_image(self: &Self, image: vk::Image, clear_color: [f32; 4]) -> &Self {
        unsafe {
            let mut clear_color_value = ClearColorValue::default();
            clear_color_value.float32 = clear_color;
            self.gpu.device.cmd_clear_color_image(
                self.command_buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &clear_color_value,
                &[vk::ImageSubresourceRange::default()
                    .layer_count(1)
                    .level_count(1)
                    .aspect_mask(vk::ImageAspectFlags::COLOR)],
            );
        }
        self
    }

    pub fn transition_image(
        self: &Self,
        image: vk::Image,
        old: ImageState,
        new: ImageState,
    ) -> &Self {
        unsafe {
            self.gpu.device.cmd_pipeline_barrier2(
                self.command_buffer,
                &vk::DependencyInfo::default().image_memory_barriers(&[
                    vk::ImageMemoryBarrier2::default()
                        .image(image)
                        .old_layout(old.layout)
                        .src_stage_mask(old.stage_mask)
                        .src_access_mask(old.access_mask)
                        .new_layout(new.layout)
                        .dst_stage_mask(new.stage_mask)
                        .dst_access_mask(new.access_mask)
                        .subresource_range(
                            vk::ImageSubresourceRange::default()
                                .layer_count(1)
                                .level_count(1)
                                .aspect_mask(vk::ImageAspectFlags::COLOR),
                        ),
                ]),
            );
        }
        self
    }
}

impl Drop for CommandEncoder {
    fn drop(&mut self) {
        unsafe {
            self.gpu
                .device
                .end_command_buffer(self.command_buffer)
                .unwrap();
        }
    }
}
