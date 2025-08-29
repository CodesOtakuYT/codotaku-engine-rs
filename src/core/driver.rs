use std::any::Any;
use std::sync::Arc;
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceExtensions, DeviceFeatures, Queue, QueueCreateInfo, QueueFlags,
};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::swapchain::{FromWindowError, Surface};
use vulkano::{Validated, Version, VulkanError, VulkanLibrary};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub struct Driver {
    pub(crate) instance: Arc<Instance>,
}

impl Driver {
    pub fn new(display: impl HasDisplayHandle) -> anyhow::Result<Self> {
        let instance = Instance::new(
            VulkanLibrary::new()?,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions: Surface::required_extensions(&display)?,
                ..Default::default()
            },
        )?;
        Ok(Self { instance })
    }

    pub fn enumerate_physical_devices(
        &self,
    ) -> Result<impl ExactSizeIterator<Item = Arc<PhysicalDevice>>, VulkanError> {
        self.instance.enumerate_physical_devices()
    }

    pub(crate) fn create_surface(
        &self,
        window: Arc<impl HasWindowHandle + HasDisplayHandle + Any + Send + Sync>,
    ) -> Result<Arc<Surface>, FromWindowError> {
        Surface::from_window(self.instance.clone(), window)
    }

    pub(crate) fn create_device(
        &self,
        physical_device: Arc<PhysicalDevice>,
        queue_family_index: u32,
    ) -> Result<(Arc<Device>, impl ExactSizeIterator<Item = Arc<Queue>>), Validated<VulkanError>>
    {
        Device::new(
            physical_device,
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: DeviceExtensions {
                    khr_swapchain: true,
                    ..DeviceExtensions::empty()
                },
                enabled_features: DeviceFeatures {
                    dynamic_rendering: true,
                    fill_mode_non_solid: true,
                    ..DeviceFeatures::empty()
                },
                ..Default::default()
            },
        )
    }

    pub fn request_device(
        &self,
        display: &impl HasDisplayHandle,
    ) -> Option<(Arc<PhysicalDevice>, u32)> {
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };
        self.enumerate_physical_devices()
            .ok()?
            .filter(|p| {
                p.api_version() >= Version::V1_3 || p.supported_extensions().khr_dynamic_rendering
            })
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.intersects(QueueFlags::GRAPHICS)
                            && p.presentation_support(i as u32, display).unwrap()
                    })
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            })
    }
}
