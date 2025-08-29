use crate::core::driver::Driver;
use crate::core::gpu::Gpu;
use crate::core::renderer::{RenderParams, Renderer};
use crate::core::swapchain_target::SwapchainTarget;
use std::collections::HashMap;
use std::sync::Arc;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

pub struct Windows {
    children: HashMap<WindowId, Vec<WindowId>>,
    swapchain_targets: HashMap<WindowId, SwapchainTarget>,
    windows: HashMap<WindowId, Arc<Window>>,
    pub gpu: Arc<Gpu>,
}

impl Windows {
    pub fn new(event_loop: &ActiveEventLoop) -> anyhow::Result<Self> {
        let driver = Arc::new(Driver::new(event_loop)?);
        let (physical_device, queue_family_index) = driver.request_device(event_loop).unwrap();
        let gpu = Arc::new(Gpu::new(driver, physical_device, queue_family_index)?);
        Self::from_gpu(gpu)
    }

    pub fn from_gpu(gpu: Arc<Gpu>) -> anyhow::Result<Self> {
        let windows = HashMap::new();
        let swapchain_targets = HashMap::new();
        let children = HashMap::new();
        Ok(Self {
            children,
            swapchain_targets,
            windows,
            gpu,
        })
    }

    pub fn add(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_attributes: WindowAttributes,
    ) -> anyhow::Result<WindowId> {
        let window = Arc::new(event_loop.create_window(window_attributes)?);
        let swapchain_target =
            SwapchainTarget::new(self.gpu.clone(), window.clone(), window.inner_size().into())?;
        let id = window.id();
        self.windows.insert(id, window);
        self.swapchain_targets.insert(id, swapchain_target);
        Ok(id)
    }

    pub fn remove(&mut self, id: WindowId) {
        self.windows.remove(&id);
        self.swapchain_targets.remove(&id);
        if let Some(children) = self.children.remove(&id) {
            for child in children {
                self.remove(child);
            }
        }
    }

    pub fn add_child(&mut self, parent: WindowId, child: WindowId) {
        if let Some(children) = self.children.get_mut(&parent) {
            children.push(child);
        } else {
            self.children.insert(parent, vec![child]);
        }
    }

    pub fn remove_child(&mut self, parent: WindowId, child: WindowId) {
        if let Some(children) = self.children.get_mut(&parent) {
            children.retain(|&c| c != child);
            if children.is_empty() {
                self.children.remove(&parent);
            }
        }
    }

    pub fn get(&self, id: WindowId) -> Option<&Arc<Window>> {
        self.windows.get(&id)
    }

    pub fn can_close(&self, id: WindowId) -> bool {
        if let Some(children) = self.children.get(&id) {
            for &child in children {
                if self.windows.contains_key(&child) {
                    return false;
                }
            }
        }
        true
    }

    pub fn redraw<Vertex>(
        &mut self,
        id: WindowId,
        renderer: &Renderer,
        render_params: RenderParams<Vertex>,
    ) -> anyhow::Result<()> {
        let swapchain_target = self.swapchain_targets.get_mut(&id).unwrap();
        let window = self.windows.get(&id).unwrap();
        if let Some(acquired) = swapchain_target.try_acquire_image(window.inner_size().into())? {
            let command_buffer = renderer.render(acquired.image_view.clone(), render_params)?;
            swapchain_target.present(acquired, command_buffer)?;
        }
        Ok(())
    }

    pub fn request_redraw(&self) {
        for window in self.windows.values() {
            window.request_redraw();
        }
    }

    pub fn resize(&mut self, id: WindowId) {
        self.swapchain_targets.get_mut(&id).unwrap().resize();
    }

    pub fn resume(&mut self) -> anyhow::Result<()> {
        for (id, window) in &self.windows {
            self.swapchain_targets.insert(
                *id,
                SwapchainTarget::new(self.gpu.clone(), window.clone(), window.inner_size().into())?,
            );
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.windows.len()
    }

    pub fn suspend(&mut self) {
        self.swapchain_targets.clear();
    }

    pub fn image_format(&self, id: WindowId) -> Option<vulkano::format::Format> {
        self.swapchain_targets.get(&id).map(|s| s.image_format())
    }
}
