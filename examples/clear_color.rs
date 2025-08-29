use codotaku_engine_rs::core::driver::Driver;
use codotaku_engine_rs::core::gpu::Gpu;
use codotaku_engine_rs::core::renderer::{RenderParams, Renderer};
use codotaku_engine_rs::graphics::windows::Windows;
use std::collections::HashMap;
use std::f32::consts::TAU;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

#[derive(Default)]
struct App {
    renderer: Option<Renderer>,
    windows: Option<Windows>,
    clear_colors: HashMap<WindowId, [f32; 4]>,
    start: Option<std::time::Instant>,
    primary_window: Option<WindowId>,
}

fn phase_from_id(id: WindowId) -> f32 {
    let mut h = DefaultHasher::new();
    id.hash(&mut h);
    let x = h.finish();
    let u = ((x ^ (x >> 32)) as u32) as f32 / (u32::MAX as f32);
    u // 0..1
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(windows) = self.windows.as_mut() {
            windows.resume().unwrap();
        } else {
            let driver = Arc::new(Driver::new(event_loop).unwrap());
            let (physical_device, queue_family_index) = driver.request_device(event_loop).unwrap();
            let gpu = Arc::new(Gpu::new(driver, physical_device, queue_family_index).unwrap());
            let mut windows = Windows::new(gpu.clone()).unwrap();
            let primary_window = windows.add(event_loop, Default::default()).unwrap();
            self.clear_colors
                .insert(primary_window, [1.0, 0.0, 0.0, 1.0]);
            for i in 0..10 {
                let window = windows.add(event_loop, Default::default()).unwrap();
                let c = i as f32 / 10.0;
                self.clear_colors.insert(window, [c, c, c, 1.0]);
                windows.add_child(primary_window, window);
            }
            self.windows = Some(windows);
            self.renderer = Some(Renderer::new(gpu));
            self.start = Some(std::time::Instant::now());
            self.primary_window = Some(primary_window);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let windows = self.windows.as_mut().unwrap();
        let renderer = self.renderer.as_ref().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                if windows.can_close(window_id) {
                    windows.remove(window_id);
                    if windows.len() == 0 {
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::RedrawRequested => windows
                .redraw(
                    window_id,
                    renderer,
                    RenderParams {
                        clear_color: self.clear_colors[&window_id],
                    },
                )
                .unwrap(),
            WindowEvent::Resized(_) => windows.resize(window_id),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let elapsed = self.start.unwrap().elapsed().as_secs_f32(); // or accumulate: self.t += delta;
        let speed_hz = 1.0; // cycles per second (tweak me)

        self.clear_colors
            .iter_mut()
            .filter(|&(&id, _)| id != self.primary_window.unwrap())
            .for_each(|(id, rgba)| {
                let hue = (phase_from_id(*id) + elapsed * speed_hz).rem_euclid(1.0);

                // smooth rainbow (branchless HSV-ish)
                let r = (TAU * (hue + 0.0)).sin().mul_add(0.5, 0.5);
                let g = (TAU * (hue + 1.0 / 3.0)).sin().mul_add(0.5, 0.5);
                let b = (TAU * (hue + 2.0 / 3.0)).sin().mul_add(0.5, 0.5);

                *rgba = [r, g, b, 1.0];
            });

        self.windows.as_ref().unwrap().request_redraw();
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.windows.as_mut().unwrap().suspend();
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut App::default())?;
    Ok(())
}
