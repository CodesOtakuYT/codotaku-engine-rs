use codotaku_engine_rs::graphics::graphics::Graphics;
use std::collections::HashMap;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

#[derive(Default)]
struct App {
    graphics: Option<Graphics>,
    clear_colors: HashMap<WindowId, [f32; 4]>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(graphics) = self.graphics.as_mut() {
            graphics.resume().unwrap();
        } else {
            let mut graphics = Graphics::new(event_loop).unwrap();
            for i in 0..1 {
                let window = graphics.add_window(event_loop, Default::default()).unwrap();
                let c = i as f32 / 1.0;
                self.clear_colors.insert(window, [c, c, c, 1.0]);
            }
            self.graphics = Some(graphics);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self
                .graphics
                .as_mut()
                .unwrap()
                .redraw_window(window_id, self.clear_colors[&window_id])
                .unwrap(),
            WindowEvent::Resized(size) => self
                .graphics
                .as_mut()
                .unwrap()
                .resize_window(window_id, size.into()),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.graphics.as_ref().unwrap().request_redraw();
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.graphics.as_mut().unwrap().suspend();
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut App::default())?;
    Ok(())
}
