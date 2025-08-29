use crate::core::driver::Driver;
use std::any::Any;
use std::sync::Arc;
use vulkano::buffer::{
    AllocateBufferError, Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer,
};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer,
};
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::Queue;
use vulkano::image::{Image, ImageUsage};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::swapchain::{FromWindowError, Surface, SurfaceInfo, Swapchain, SwapchainCreateInfo};
use vulkano::sync::GpuFuture;
use vulkano::{sync, Validated, VulkanError};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub struct Gpu {
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    pub queue: Arc<Queue>,
    driver: Arc<Driver>,
}

impl Gpu {
    pub fn new(
        driver: Arc<Driver>,
        physical_device: Arc<PhysicalDevice>,
        queue_family_index: u32,
    ) -> anyhow::Result<Self> {
        let d = driver.clone();
        let (device, mut queues) = d.create_device(physical_device, queue_family_index)?;
        let queue = queues.next().unwrap();
        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            Default::default(),
        ));

        Ok(Gpu {
            command_buffer_allocator,
            memory_allocator,
            queue,
            driver,
        })
    }

    pub(crate) fn create_surface(
        &self,
        window: Arc<impl HasWindowHandle + HasDisplayHandle + Any + Send + Sync>,
    ) -> Result<Arc<Surface>, FromWindowError> {
        self.driver.create_surface(window)
    }

    pub(crate) fn create_swapchain(
        &self,
        surface: Arc<Surface>,
        image_extent: [u32; 2],
        image_usage: ImageUsage,
    ) -> Result<(Arc<Swapchain>, Vec<Arc<Image>>), Validated<VulkanError>> {
        let surface_capabilities = self
            .queue
            .device()
            .physical_device()
            .surface_capabilities(&surface, SurfaceInfo::default())?;

        let (image_format, _) = self
            .queue
            .device()
            .physical_device()
            .surface_formats(&surface, Default::default())?[0];

        Swapchain::new(
            self.queue.device().clone(),
            surface,
            SwapchainCreateInfo {
                min_image_count: surface_capabilities.min_image_count.max(2),
                image_format,
                image_extent,
                image_usage,
                composite_alpha: surface_capabilities
                    .supported_composite_alpha
                    .into_iter()
                    .next()
                    .unwrap(),

                ..Default::default()
            },
        )
    }

    pub(crate) fn now(&self) -> Box<dyn GpuFuture> {
        sync::now(self.queue.device().clone()).boxed()
    }

    pub(crate) fn create_command_buffer_builder(
        &self,
    ) -> Result<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, Validated<VulkanError>> {
        AutoCommandBufferBuilder::primary(
            self.command_buffer_allocator.clone(),
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
    }

    pub(crate) fn create_buffer<T, I>(
        &self,
        data: I,
        usage: BufferUsage,
    ) -> Result<Subbuffer<[T]>, Validated<AllocateBufferError>>
    where
        T: BufferContents,
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        Buffer::from_iter(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            data,
        )
    }
}
