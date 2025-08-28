use crate::core::driver::Driver;
use ash::prelude::VkResult;
use ash::vk;
use std::sync::Arc;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub(crate) struct Gpu {
    command_buffers: Vec<vk::CommandBuffer>,
    command_pool: vk::CommandPool,
    queue: vk::Queue,
    pub(crate) swapchain_extension: ash::khr::swapchain::Device,
    push_descriptor_extension: ash::khr::push_descriptor::Device,
    pub(crate) device: ash::Device,
    physical_device: vk::PhysicalDevice,
    driver: Arc<Driver>,
}

impl Gpu {
    pub(crate) fn new(
        driver: Arc<Driver>,
        physical_device: vk::PhysicalDevice,
    ) -> anyhow::Result<Self> {
        unsafe {
            let device = driver.create_device(physical_device)?;
            let swapchain_extension = driver.load_swapchain_extension(&device);
            let push_descriptor_extension = driver.load_push_descriptor_extension(&device);

            let queue = device.get_device_queue(0, 0);
            let command_pool = device.create_command_pool(
                &vk::CommandPoolCreateInfo::default().flags(
                    vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
                        | vk::CommandPoolCreateFlags::TRANSIENT,
                ),
                None,
            )?;
            let command_buffers = device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(command_pool)
                    .command_buffer_count(1),
            )?;

            Ok(Gpu {
                driver,
                physical_device,
                device,
                queue,
                command_pool,
                command_buffers,
                swapchain_extension,
                push_descriptor_extension,
            })
        }
    }

    pub(crate) fn create_surface(
        &self,
        window: impl HasDisplayHandle + HasWindowHandle,
    ) -> anyhow::Result<vk::SurfaceKHR> {
        self.driver.create_surface(window)
    }

    pub(crate) fn create_swapchain(
        &self,
        surface: vk::SurfaceKHR,
        extent: vk::Extent2D,
        old_swapchain: vk::SwapchainKHR,
    ) -> anyhow::Result<vk::SwapchainKHR> {
        unsafe {
            let surface_capabilities = self
                .driver
                .get_surface_capabilities(self.physical_device, surface)?;
            let swapchain = self.swapchain_extension.create_swapchain(
                &vk::SwapchainCreateInfoKHR::default()
                    .surface(surface)
                    .old_swapchain(old_swapchain)
                    .clipped(true)
                    .image_array_layers(1)
                    .image_format(vk::Format::R8G8B8A8_SRGB)
                    .image_extent(extent)
                    .min_image_count(surface_capabilities.min_image_count + 1)
                    .pre_transform(surface_capabilities.current_transform)
                    .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                    .image_usage(vk::ImageUsageFlags::TRANSFER_DST),
                None,
            )?;
            Ok(swapchain)
        }
    }

    pub(crate) fn create_command_pool(&self) -> anyhow::Result<vk::CommandPool> {
        unsafe {
            Ok(self.device.create_command_pool(
                &vk::CommandPoolCreateInfo::default().flags(
                    vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
                        | vk::CommandPoolCreateFlags::TRANSIENT,
                ),
                None,
            )?)
        }
    }

    pub(crate) fn create_semaphore(&self) -> anyhow::Result<vk::Semaphore> {
        unsafe {
            Ok(self
                .device
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?)
        }
    }

    pub(crate) fn create_fence(&self) -> anyhow::Result<vk::Fence> {
        unsafe {
            Ok(self.device.create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?)
        }
    }

    pub(crate) fn allocate_command_buffers(
        &self,
        command_pool: vk::CommandPool,
        command_buffer_count: u32,
    ) -> anyhow::Result<Vec<vk::CommandBuffer>> {
        unsafe {
            Ok(self.device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(command_pool)
                    .command_buffer_count(command_buffer_count),
            )?)
        }
    }

    pub(crate) fn get_swapchain_images(
        &self,
        swapchain: vk::SwapchainKHR,
    ) -> anyhow::Result<Vec<vk::Image>> {
        unsafe { Ok(self.swapchain_extension.get_swapchain_images(swapchain)?) }
    }

    pub(crate) fn wait_fences(&self, fences: &[vk::Fence]) -> anyhow::Result<()> {
        unsafe { Ok(self.device.wait_for_fences(fences, true, u64::MAX)?) }
    }

    pub(crate) fn reset_fences(&self, fences: &[vk::Fence]) -> anyhow::Result<()> {
        unsafe { Ok(self.device.reset_fences(fences)?) }
    }

    pub(crate) fn acquire_image(
        &self,
        swapchain: vk::SwapchainKHR,
        semaphore: vk::Semaphore,
        fence: vk::Fence,
    ) -> VkResult<(u32, bool)> {
        unsafe {
            self.swapchain_extension
                .acquire_next_image(swapchain, u64::MAX, semaphore, fence)
        }
    }

    pub(crate) fn present_image(
        &self,
        swapchain: vk::SwapchainKHR,
        image_index: u32,
        wait_semaphores: &[vk::Semaphore],
    ) -> VkResult<bool> {
        unsafe {
            self.swapchain_extension.queue_present(
                self.queue,
                &vk::PresentInfoKHR::default()
                    .swapchains(&[swapchain])
                    .image_indices(&[image_index])
                    .wait_semaphores(wait_semaphores),
            )
        }
    }

    pub(crate) fn submit_command_buffers(
        &self,
        command_buffers: &[vk::CommandBuffer],
        wait_semaphores: &[vk::Semaphore],
        signal_semaphores: &[vk::Semaphore],
        fence: vk::Fence,
    ) -> anyhow::Result<()> {
        unsafe {
            Ok(self.device.queue_submit(
                self.queue,
                &[vk::SubmitInfo::default()
                    .command_buffers(command_buffers)
                    .wait_semaphores(wait_semaphores)
                    .signal_semaphores(signal_semaphores)
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::TRANSFER])],
                fence,
            )?)
        }
    }
}

impl Drop for Gpu {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
        }
    }
}
