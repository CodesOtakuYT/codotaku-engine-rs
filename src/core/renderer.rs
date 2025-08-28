use crate::core::command_encoder::{CommandEncoder, ImageState};
use crate::core::gpu::Gpu;
use ash::vk;
use std::sync::Arc;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub(crate) struct Renderer {
    gpu: Arc<Gpu>,
    surface: vk::SurfaceKHR,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_acquired_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    swapchain_image_fences: Vec<vk::Fence>,
    frame_index: usize,
    is_swapchain_dirty: bool,
    swapchain_extent: vk::Extent2D,
    swapchain_image_index: usize,
}

impl Renderer {
    pub(crate) fn new(
        gpu: Arc<Gpu>,
        window_handle: impl HasWindowHandle + HasDisplayHandle,
        extent: [u32; 2],
    ) -> anyhow::Result<Self> {
        let surface = gpu.create_surface(window_handle)?;
        let swapchain_extent = vk::Extent2D::default().width(extent[0]).height(extent[1]);
        let swapchain = gpu.create_swapchain(
            surface,
            vk::Extent2D::default().width(extent[0]).height(extent[1]),
            vk::SwapchainKHR::null(),
        )?;
        let swapchain_images = gpu.get_swapchain_images(swapchain)?;
        let command_pool = gpu.create_command_pool()?;
        let command_buffers =
            gpu.allocate_command_buffers(command_pool, swapchain_images.len() as u32)?;
        let image_acquired_semaphores = (0..2)
            .map(|_| gpu.create_semaphore())
            .collect::<Result<Vec<_>, _>>()?;
        let render_finished_semaphores = (0..swapchain_images.len() as u32)
            .map(|_| gpu.create_semaphore())
            .collect::<Result<Vec<_>, _>>()?;
        let in_flight_fences = (0..2)
            .map(|_| gpu.create_fence())
            .collect::<Result<Vec<_>, _>>()?;
        let swapchain_image_fences = swapchain_images
            .iter()
            .map(|_| vk::Fence::null())
            .collect::<Vec<_>>();
        let frame_index = 0;
        Ok(Self {
            gpu,
            surface,
            swapchain,
            swapchain_images,
            command_pool,
            command_buffers,
            image_acquired_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            swapchain_image_fences,
            frame_index,
            is_swapchain_dirty: false,
            swapchain_extent,
            swapchain_image_index: 0,
        })
    }

    pub(crate) fn render(&mut self, clear_color: [f32; 4]) -> anyhow::Result<bool> {
        let in_flight_fence = self.in_flight_fences[self.frame_index];
        self.gpu
            .wait_fences(&[self.in_flight_fences[self.frame_index]])?;

        if (self.is_swapchain_dirty) {
            unsafe {
                self.gpu.device.device_wait_idle()?;
            }
            let old_swapchain = self.swapchain;
            self.swapchain =
                self.gpu
                    .create_swapchain(self.surface, self.swapchain_extent, old_swapchain)?;
            unsafe {
                self.gpu
                    .swapchain_extension
                    .destroy_swapchain(old_swapchain, None);
            }
            self.swapchain_images = self.gpu.get_swapchain_images(self.swapchain)?;
            self.is_swapchain_dirty = false;
            return Ok(false);
        }

        let (image_index, _is_suboptimal) = self.gpu.acquire_image(
            self.swapchain,
            self.image_acquired_semaphores[self.frame_index],
            vk::Fence::null(),
        )?;

        self.swapchain_image_index = image_index as usize;

        self.gpu
            .reset_fences(&[self.in_flight_fences[self.frame_index]])?;

        let image_fence = &mut self.swapchain_image_fences[image_index as usize];
        if (*image_fence != vk::Fence::null()) {
            self.gpu.wait_fences(&[*image_fence])?;
        }
        *image_fence = in_flight_fence;

        let swapchain_image = self.swapchain_images[image_index as usize];

        {
            let command_encoder =
                CommandEncoder::new(self.gpu.clone(), self.command_buffers[image_index as usize])?;

            command_encoder
                .transition_image(swapchain_image, ImageState::none(), ImageState::clear())
                .clear_image(swapchain_image, clear_color)
                .transition_image(swapchain_image, ImageState::clear(), ImageState::present());
        }

        self.gpu.submit_command_buffers(
            &[self.command_buffers[image_index as usize]],
            &[self.image_acquired_semaphores[self.frame_index]],
            &[self.render_finished_semaphores[image_index as usize]],
            self.in_flight_fences[self.frame_index],
        )?;
        Ok(true)
    }

    pub(crate) fn present(&mut self) -> anyhow::Result<()> {
        self.gpu.present_image(
            self.swapchain,
            self.swapchain_image_index as u32,
            &[self.render_finished_semaphores[self.swapchain_image_index as usize]],
        )?;

        self.frame_index = (self.frame_index + 1) % 2;
        Ok(())
    }

    pub(crate) fn resize(&mut self, size: [u32; 2]) {
        self.is_swapchain_dirty = true;
        self.swapchain_extent = vk::Extent2D::default().width(size[0]).height(size[1]);
    }
}
