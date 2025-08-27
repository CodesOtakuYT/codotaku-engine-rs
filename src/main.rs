use ash::vk;
use ash::vk::{ClearColorValue, Extent2D};
use ash_window::{create_surface, enumerate_required_extensions};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::{Window, WindowAttributes, WindowId};

struct Gpu {
    entry: ash::Entry,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,

    surface_extension: ash::khr::surface::Instance,
    swapchain_extension: ash::khr::swapchain::Device,
}

impl Gpu {
    fn new(display_handle: impl HasDisplayHandle) -> anyhow::Result<Self> {
        unsafe {
            let entry = ash::Entry::load()?;
            let required_extensions =
                enumerate_required_extensions(display_handle.display_handle()?.as_raw())?;
            let instance = entry.create_instance(
                &vk::InstanceCreateInfo::default()
                    .enabled_extension_names(required_extensions)
                    .application_info(
                        &vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_3),
                    ),
                None,
            )?;

            let surface_extension = ash::khr::surface::Instance::new(&entry, &instance);

            let physical_device = instance
                .enumerate_physical_devices()?
                .into_iter()
                .next()
                .unwrap();

            let device = instance.create_device(
                physical_device,
                &vk::DeviceCreateInfo::default()
                    .enabled_extension_names(&[ash::khr::swapchain::NAME.as_ptr()])
                    .queue_create_infos(&[
                        vk::DeviceQueueCreateInfo::default().queue_priorities(&[1.0_f32])
                    ]),
                None,
            )?;

            let swapchain_extension = ash::khr::swapchain::Device::new(&instance, &device);

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

            Ok(Self {
                entry,
                instance,
                physical_device,
                device,
                queue,
                command_pool,
                command_buffers,
                surface_extension,
                swapchain_extension,
            })
        }
    }

    fn create_surface(
        &self,
        display_handle: impl HasDisplayHandle + HasWindowHandle,
    ) -> anyhow::Result<vk::SurfaceKHR> {
        unsafe {
            Ok(create_surface(
                &self.entry,
                &self.instance,
                display_handle.display_handle().unwrap().as_raw(),
                display_handle.window_handle()?.as_raw(),
                None,
            )?)
        }
    }

    fn create_swapchain(
        &self,
        surface: vk::SurfaceKHR,
        extent: Extent2D,
    ) -> anyhow::Result<vk::SwapchainKHR> {
        unsafe {
            let surface_capabilities = self
                .surface_extension
                .get_physical_device_surface_capabilities(self.physical_device, surface)?;
            let swapchain = self.swapchain_extension.create_swapchain(
                &vk::SwapchainCreateInfoKHR::default()
                    .surface(surface)
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

    fn create_command_pool(&self) -> anyhow::Result<vk::CommandPool> {
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

    fn create_semaphore(&self) -> anyhow::Result<vk::Semaphore> {
        unsafe {
            Ok(self
                .device
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?)
        }
    }

    fn create_fence(&self) -> anyhow::Result<vk::Fence> {
        unsafe {
            Ok(self.device.create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?)
        }
    }

    fn allocate_command_buffers(
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

    fn get_swapchain_images(&self, swapchain: vk::SwapchainKHR) -> anyhow::Result<Vec<vk::Image>> {
        unsafe { Ok(self.swapchain_extension.get_swapchain_images(swapchain)?) }
    }

    fn wait_fences(&self, fences: &[vk::Fence]) -> anyhow::Result<()> {
        unsafe { Ok(self.device.wait_for_fences(fences, true, u64::MAX)?) }
    }

    fn reset_fences(&self, fences: &[vk::Fence]) -> anyhow::Result<()> {
        unsafe { Ok(self.device.reset_fences(fences)?) }
    }

    fn acquire_image(
        &self,
        swapchain: vk::SwapchainKHR,
        semaphore: vk::Semaphore,
        fence: vk::Fence,
    ) -> anyhow::Result<(u32, bool)> {
        unsafe {
            Ok(self.swapchain_extension.acquire_next_image(
                swapchain,
                u64::MAX,
                semaphore,
                fence,
            )?)
        }
    }

    fn present_image(
        &self,
        swapchain: vk::SwapchainKHR,
        image_index: u32,
        wait_semaphores: &[vk::Semaphore],
    ) -> anyhow::Result<bool> {
        unsafe {
            Ok(self.swapchain_extension.queue_present(
                self.queue,
                &vk::PresentInfoKHR::default()
                    .swapchains(&[swapchain])
                    .image_indices(&[image_index])
                    .wait_semaphores(wait_semaphores),
            )?)
        }
    }

    fn submit_command_buffers(
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
                    .signal_semaphores(signal_semaphores)],
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
            self.instance.destroy_instance(None);
        }
    }
}

struct Renderer {
    gpu: Arc<Gpu>,
    surface: vk::SurfaceKHR,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_acquired_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    fences: Vec<vk::Fence>,
    frame_index: usize,
}

impl Renderer {
    fn new(
        gpu: Arc<Gpu>,
        window_handle: impl HasWindowHandle + HasDisplayHandle,
        extent: Extent2D,
    ) -> anyhow::Result<Self> {
        let surface = gpu.create_surface(window_handle)?;
        let swapchain = gpu.create_swapchain(surface, extent)?;
        let swapchain_images = gpu.get_swapchain_images(swapchain)?;
        let command_pool = gpu.create_command_pool()?;
        let command_buffers = gpu.allocate_command_buffers(command_pool, 2)?;
        let image_acquired_semaphores = (0..2)
            .map(|_| gpu.create_semaphore())
            .collect::<Result<Vec<_>, _>>()?;
        let render_finished_semaphores = (0..2)
            .map(|_| gpu.create_semaphore())
            .collect::<Result<Vec<_>, _>>()?;
        let fences = (0..2)
            .map(|_| gpu.create_fence())
            .collect::<Result<Vec<_>, _>>()?;
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
            fences,
            frame_index,
        })
    }

    fn render(&mut self, clear_color: [f32; 4]) -> anyhow::Result<()> {
        self.gpu.wait_fences(&[self.fences[self.frame_index]])?;
        let (image_index, _is_suboptimal) = self.gpu.acquire_image(
            self.swapchain,
            self.image_acquired_semaphores[self.frame_index],
            self.fences[self.frame_index],
        )?;
        self.gpu.reset_fences(&[self.fences[self.frame_index]])?;

        unsafe {
            let command_buffer = self.command_buffers[self.frame_index];
            let device = &self.gpu.device;
            device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::default())?;
            device.begin_command_buffer(
                command_buffer,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;
            let mut clear_color_value = ClearColorValue::default();
            clear_color_value.float32 = clear_color;
            device.cmd_clear_color_image(
                command_buffer,
                self.swapchain_images[image_index as usize],
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &clear_color_value,
                &[vk::ImageSubresourceRange::default()
                    .layer_count(1)
                    .level_count(1)
                    .aspect_mask(vk::ImageAspectFlags::COLOR)],
            );
            device.end_command_buffer(command_buffer)?;
        }

        self.gpu.submit_command_buffers(
            &[self.command_buffers[self.frame_index]],
            &[self.image_acquired_semaphores[self.frame_index]],
            &[self.render_finished_semaphores[self.frame_index]],
            self.fences[self.frame_index],
        )?;

        self.gpu.present_image(
            self.swapchain,
            image_index,
            &[self.render_finished_semaphores[self.frame_index]],
        )?;

        self.frame_index = (self.frame_index + 1) % 2;
        Ok(())
    }
}

#[derive(Default)]
struct App {
    window: Option<Window>,
    window2: Option<Window>,
    gpu: Option<Arc<Gpu>>,
    renderer: Option<Renderer>,
    renderer2: Option<Renderer>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(WindowAttributes::default())
            .unwrap();
        let window2 = event_loop
            .create_window(WindowAttributes::default())
            .unwrap();
        let gpu = Arc::new(Gpu::new(event_loop).unwrap());
        let window_size = window.inner_size();
        self.renderer = Some(
            Renderer::new(
                gpu.clone(),
                &window,
                Extent2D::default()
                    .width(window_size.width)
                    .height(window_size.height),
            )
            .unwrap(),
        );
        self.renderer2 = Some(
            Renderer::new(
                gpu.clone(),
                &window2,
                Extent2D::default()
                    .width(window_size.width)
                    .height(window_size.height),
            )
            .unwrap(),
        );
        self.window = Some(window);
        self.window2 = Some(window2);
        self.gpu = Some(gpu);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.render([1.0, 0.0, 0.0, 1.0]).unwrap();
                }
                if let Some(renderer2) = self.renderer2.as_mut() {
                    renderer2.render([0.0, 1.0, 0.0, 1.0]).unwrap();
                }
            }
            _ => {}
        }
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut App::default())?;
    Ok(())
}
