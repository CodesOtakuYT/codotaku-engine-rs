use codotaku_engine_rs::core::driver::Driver;
use codotaku_engine_rs::core::gpu::Gpu;
use codotaku_engine_rs::core::renderer::{Mesh, RenderParams, Renderer};
use codotaku_engine_rs::graphics::windows::Windows;
use lyon::geom::{point, Box2D};
use lyon::lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex};
use lyon::path::builder::BorderRadii;
use lyon::path::{Path, Winding};
use lyon::tessellation::VertexBuffers;
use std::collections::HashMap;
use std::sync::Arc;
use vulkano::buffer::BufferContents;
use vulkano::pipeline::graphics::vertex_input::Vertex as VertexTrait;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

#[derive(Default)]
struct App {
    mesh: Option<Mesh<Vertex>>,
    renderer: Option<Renderer>,
    windows: Option<Windows>,
    clear_colors: HashMap<WindowId, [f32; 4]>,
    start: Option<std::time::Instant>,
}

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
                        f_color = vec4(1.0, 0.0, 0.0, 1.0);
                    }
                ",
    }
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
            for _ in 0..2 {
                let window = windows.add(event_loop, Default::default()).unwrap();
                self.clear_colors.insert(window, [0.1, 0.1, 0.1, 1.0]);
            }
            let window_id = self.clear_colors.keys().next().unwrap();
            let image_format = windows.image_format(*window_id).unwrap();
            let vs = vs::load(gpu.queue.device().clone())
                .unwrap()
                .entry_point("main")
                .unwrap();
            let fs = fs::load(gpu.queue.device().clone())
                .unwrap()
                .entry_point("main")
                .unwrap();

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
                tessellator
                    .tessellate_path(
                        &path,
                        &FillOptions::default().with_tolerance(0.01),
                        &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| Vertex {
                            position: vertex.position().to_array(),
                        }),
                    )
                    .unwrap();
            }

            let mesh = Mesh::new(gpu.clone(), geometry.vertices, geometry.indices).unwrap();

            self.renderer = Some(Renderer::new::<Vertex>(gpu, image_format, vs, fs).unwrap());
            self.start = Some(std::time::Instant::now());
            self.windows = Some(windows);
            self.mesh = Some(mesh);
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
                        mesh: self.mesh.clone(),
                    },
                )
                .unwrap(),
            WindowEvent::Resized(_) => windows.resize(window_id),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
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
