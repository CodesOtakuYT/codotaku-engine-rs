use crate::core::driver::Driver;
use crate::core::gpu::Gpu;
use crate::core::renderer::Renderer;
use std::collections::HashMap;
use std::sync::Arc;
use winit::event_loop::ActiveEventLoop;
use winit::raw_window_handle::HasDisplayHandle;
use winit::window::{Window, WindowAttributes, WindowId};

pub struct Graphics {
    renderers: HashMap<WindowId, Renderer>,
    windows: HashMap<WindowId, Window>,
    gpu: Arc<Gpu>,
}

impl Graphics {
    pub fn new(display: impl HasDisplayHandle) -> anyhow::Result<Self> {
        let driver = Arc::new(Driver::new(display)?);
        let physical_device = driver
            .enumerate_physical_devices()?
            .into_iter()
            .next()
            .unwrap();
        let gpu = Arc::new(Gpu::new(driver, physical_device)?);
        let windows = HashMap::new();
        let renderers = HashMap::new();
        Ok(Self {
            renderers,
            windows,
            gpu,
        })
    }

    pub fn add_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_attributes: WindowAttributes,
    ) -> anyhow::Result<WindowId> {
        let window = event_loop.create_window(window_attributes)?;
        let renderer = Renderer::new(self.gpu.clone(), &window, window.inner_size().into())?;
        let id = window.id();
        self.windows.insert(id, window);
        self.renderers.insert(id, renderer);
        Ok(id)
    }

    pub fn remove_window(&mut self, id: WindowId) {
        self.windows.remove(&id);
        self.renderers.remove(&id);
    }

    pub fn redraw_window(&mut self, id: WindowId, clear_color: [f32; 4]) -> anyhow::Result<()> {
        let renderer = self.renderers.get_mut(&id).unwrap();
        if (renderer.render(clear_color)?) {
            self.windows.get(&id).unwrap().pre_present_notify();
            renderer.present()?;
        }
        Ok(())
    }

    pub fn request_redraw(&self) {
        for window in self.windows.values() {
            window.request_redraw();
        }
    }

    pub fn resize_window(&mut self, id: WindowId, size: [u32; 2]) {
        self.renderers.get_mut(&id).unwrap().resize(size);
    }

    pub fn resume(&mut self) -> anyhow::Result<()> {
        for (id, window) in &self.windows {
            self.renderers.insert(
                *id,
                Renderer::new(self.gpu.clone(), window, window.inner_size().into())?,
            );
        }
        Ok(())
    }

    pub fn suspend(&mut self) {
        self.renderers.clear();
    }
}
