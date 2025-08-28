use ash::vk;
use ash_window::{create_surface, enumerate_required_extensions};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub(crate) struct Driver {
    entry: ash::Entry,
    instance: ash::Instance,
    surface_extension: ash::khr::surface::Instance,
}

impl Driver {
    pub(crate) fn new(display: impl HasDisplayHandle) -> anyhow::Result<Self> {
        unsafe {
            let entry = ash::Entry::load()?;
            let required_extensions =
                enumerate_required_extensions(display.display_handle()?.as_raw())?;
            let instance = entry.create_instance(
                &vk::InstanceCreateInfo::default()
                    .enabled_extension_names(required_extensions)
                    .application_info(
                        &vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_3),
                    ),
                None,
            )?;
            let surface_extension = ash::khr::surface::Instance::new(&entry, &instance);
            Ok(Self {
                entry,
                instance,
                surface_extension,
            })
        }
    }

    pub(crate) fn enumerate_physical_devices(
        self: &Self,
    ) -> anyhow::Result<Vec<vk::PhysicalDevice>> {
        unsafe { Ok(self.instance.enumerate_physical_devices()?) }
    }

    pub(crate) fn load_swapchain_extension(
        self: &Self,
        device: &ash::Device,
    ) -> ash::khr::swapchain::Device {
        ash::khr::swapchain::Device::new(&self.instance, device)
    }

    pub(crate) fn load_push_descriptor_extension(
        self: &Self,
        device: &ash::Device,
    ) -> ash::khr::push_descriptor::Device {
        ash::khr::push_descriptor::Device::new(&self.instance, device)
    }

    pub(crate) fn create_surface(
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

    pub(crate) fn create_device(
        self: &Self,
        physical_device: vk::PhysicalDevice,
    ) -> anyhow::Result<ash::Device> {
        unsafe {
            Ok(self.instance.create_device(
                physical_device,
                &vk::DeviceCreateInfo::default()
                    .enabled_extension_names(&[
                        ash::khr::swapchain::NAME.as_ptr(),
                        ash::khr::push_descriptor::NAME.as_ptr(),
                    ])
                    .queue_create_infos(&[
                        vk::DeviceQueueCreateInfo::default().queue_priorities(&[1.0_f32])
                    ])
                    .push_next(
                        &mut vk::PhysicalDeviceVulkan12Features::default()
                            .buffer_device_address(true)
                            .scalar_block_layout(true)
                            .timeline_semaphore(true),
                    )
                    .push_next(
                        &mut vk::PhysicalDeviceVulkan13Features::default()
                            .synchronization2(true)
                            .dynamic_rendering(true),
                    ),
                None,
            )?)
        }
    }

    pub(crate) fn get_surface_capabilities(
        &self,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
    ) -> anyhow::Result<vk::SurfaceCapabilitiesKHR> {
        unsafe {
            Ok(self
                .surface_extension
                .get_physical_device_surface_capabilities(physical_device, surface)?)
        }
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}
