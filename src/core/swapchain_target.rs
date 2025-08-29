use crate::core::gpu::Gpu;
use anyhow::anyhow;
use std::any::Any;
use std::sync::Arc;
use vulkano::command_buffer::PrimaryCommandBufferAbstract;
use vulkano::device::DeviceOwned;
use vulkano::image::{Image, ImageUsage};
use vulkano::swapchain::{
    acquire_next_image, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo,
    SwapchainPresentInfo,
};
use vulkano::sync::GpuFuture;
use vulkano::{sync, Validated, VulkanError};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub struct Acquired {
    pub(crate) image: Arc<Image>,
    image_index: u32,
    acquire_future: SwapchainAcquireFuture,
}

pub(crate) struct SwapchainTarget {
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    swapchain_images: Vec<Arc<Image>>,
    swapchain: Arc<Swapchain>,
    gpu: Arc<Gpu>,
}

impl SwapchainTarget {
    pub(crate) fn new(
        gpu: Arc<Gpu>,
        window: Arc<impl HasWindowHandle + HasDisplayHandle + Any + Send + Sync>,
        extent: [u32; 2],
    ) -> anyhow::Result<Self> {
        let surface = gpu.create_surface(window)?;
        let (swapchain, swapchain_images) =
            gpu.create_swapchain(surface, extent, ImageUsage::TRANSFER_DST)?;
        let previous_frame_end = Some(gpu.now());
        Ok(Self {
            recreate_swapchain: false,
            gpu,
            swapchain,
            swapchain_images,
            previous_frame_end,
        })
    }

    pub(crate) fn try_acquire_image(
        &mut self,
        window_size: [u32; 2],
    ) -> anyhow::Result<Option<Acquired>> {
        if window_size[0] == 0 || window_size[1] == 0 {
            return Ok(None);
        }

        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        if self.recreate_swapchain {
            let (new_swapchain, new_images) = self.swapchain.recreate(SwapchainCreateInfo {
                image_extent: window_size.into(),
                ..self.swapchain.create_info()
            })?;

            self.swapchain = new_swapchain;
            self.swapchain_images = new_images;
            self.recreate_swapchain = false;
        }

        let (image_index, suboptimal, acquire_future) =
            match acquire_next_image(self.swapchain.clone(), None).map_err(Validated::unwrap) {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return Ok(None);
                }
                Err(e) => return Err(anyhow!(e)),
            };

        if suboptimal {
            self.recreate_swapchain = true;
        }

        Ok(Some(Acquired {
            image: self.swapchain_images[image_index as usize].clone(),
            image_index,
            acquire_future,
        }))
    }

    pub(crate) fn present(
        &mut self,
        acquired: Acquired,
        command_buffer: Arc<impl PrimaryCommandBufferAbstract + 'static>,
    ) -> anyhow::Result<bool> {
        let future = self
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquired.acquire_future)
            .then_execute(self.gpu.queue.clone(), command_buffer)?
            .then_swapchain_present(
                self.gpu.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(
                    self.swapchain.clone(),
                    acquired.image_index,
                ),
            )
            .then_signal_fence_and_flush();

        match future.map_err(Validated::unwrap) {
            Ok(future) => {
                self.previous_frame_end = Some(future.boxed());
                Ok(true)
            }
            Err(VulkanError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end = Some(sync::now(self.gpu.queue.device().clone()).boxed());
                Ok(true)
            }
            Err(e) => {
                println!("failed to flush future: {e}");
                self.previous_frame_end = Some(sync::now(self.gpu.queue.device().clone()).boxed());
                Err(anyhow!(e))
            }
        }
    }

    pub(crate) fn resize(&mut self) {
        self.recreate_swapchain = true;
    }
}
