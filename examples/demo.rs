use codotaku_engine_rs::core::renderer::{Mesh, RenderParams, Renderer};
use codotaku_engine_rs::graphics::windows::Windows;
use lyon::geom::{point, Box2D};
use lyon::lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex};
use lyon::path::builder::BorderRadii;
use lyon::path::{Path, Winding};
use lyon::tessellation::VertexBuffers;
use std::collections::HashMap;
use vulkano::buffer::BufferContents;
use vulkano::pipeline::graphics::vertex_input::Vertex as VertexTrait;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

#[derive(BufferContents, VertexTrait, Clone)]
#[repr(C)]
struct Vertex {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2],
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
                    #version 450

                    layout(location = 0) in vec2 position;

                    void main() {
                        gl_Position = vec4(position, 0.0, 1.0);
                    }
                ",
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
                    #version 450

                    layout(location = 0) out vec4 f_color;

                    void main() {
                        f_color = vec4(1.0, 1.0, 1.0, 1.0);
                    }
                ",
    }
}

struct Graphics {
    mesh: Mesh<Vertex>,
    renderer: Renderer,
    windows: Windows,
    clear_colors: HashMap<WindowId, [f32; 4]>,
}

impl Graphics {
    fn new(event_loop: &ActiveEventLoop) -> anyhow::Result<Self> {
        let mut windows = Windows::new(event_loop)?;
        let gpu = windows.gpu.clone();
        let mut clear_colors = HashMap::new();
        for _ in 0..2 {
            let window = windows.add(event_loop, Default::default())?;
            clear_colors.insert(window, [0.1, 0.1, 0.1, 1.0]);
        }
        let window_id = clear_colors.keys().next().unwrap();
        let image_format = windows.image_format(*window_id).unwrap();
        let vs = vs::load(gpu.queue.device().clone())?
            .entry_point("main")
            .unwrap();
        let fs = fs::load(gpu.queue.device().clone())?
            .entry_point("main")
            .unwrap();

        let renderer = Renderer::new::<Vertex>(gpu.clone(), image_format, vs, fs)?;

        let mut builder = Path::builder();
        builder.add_rounded_rectangle(
            &Box2D {
                min: point(0.0, 0.0),
                max: point(100.0 / 100.0, 50.0 / 100.0),
            },
            &BorderRadii {
                top_left: 10.0,
                top_right: 5.0,
                bottom_left: 20.0,
                bottom_right: 25.0,
            },
            Winding::Positive,
        );
        let path = builder.build();

        let mut geometry: VertexBuffers<Vertex, u16> = VertexBuffers::new();

        let mut tessellator = FillTessellator::new();

        {
            tessellator.tessellate_path(
                &path,
                &FillOptions::default().with_tolerance(0.01),
                &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| Vertex {
                    position: vertex.position().to_array(),
                }),
            )?;
        }

        let mesh = Mesh::new(gpu.clone(), geometry.vertices, geometry.indices)?;

        Ok(Self {
            mesh,
            renderer,
            windows,
            clear_colors,
        })
    }

    fn resume(&mut self) -> anyhow::Result<()> {
        self.windows.resume()
    }

    fn suspend(&mut self) {
        self.windows.suspend()
    }

    fn about_to_wait(&self) {
        self.windows.request_redraw();
    }

    fn close_requested(&mut self, window_id: WindowId) -> bool {
        let windows = &mut self.windows;
        if windows.can_close(window_id) {
            windows.remove(window_id);
            if windows.len() == 0 {
                return true;
            }
        }
        false
    }

    fn redraw_requested(&mut self, window_id: WindowId) {
        self.windows
            .redraw(
                window_id,
                &self.renderer,
                RenderParams {
                    clear_color: self.clear_colors[&window_id],
                    mesh: Some(self.mesh.clone()),
                },
            )
            .unwrap()
    }

    fn resize_window(&mut self, window_id: WindowId) {
        self.windows.resize(window_id);
    }
}

#[derive(Default)]
struct App {
    graphics: Option<Graphics>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(graphics) = self.graphics.as_mut() {
            graphics.resume().unwrap();
        } else {
            let graphics = Graphics::new(event_loop);
            self.graphics = Some(graphics.unwrap());
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let graphics = self.graphics.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                if graphics.close_requested(window_id) {
                    event_loop.exit()
                }
            }
            WindowEvent::RedrawRequested => graphics.redraw_requested(window_id),
            WindowEvent::Resized(_) => graphics.resize_window(window_id),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.graphics.as_ref().unwrap().about_to_wait();
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
