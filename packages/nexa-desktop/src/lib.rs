use nexa_core::Runtime;
use std::sync::Arc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[cfg(feature = "gpu")]
use nexa_renderer_gpu::GpuRenderer;

pub struct DesktopApp {
    title: String,
}

impl DesktopApp {
    pub fn new() -> Self {
        Self {
            title: "Nexa Desktop App".to_string(),
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn run(self) {
        let event_loop = EventLoop::new().unwrap();
        let window = Arc::new(
            WindowBuilder::new()
                .with_title(&self.title)
                .build(&event_loop)
                .unwrap(),
        );

        // Initialize Runtime
        let mut runtime = Runtime::new();

        // Initialize GPU Renderer if enabled
        #[cfg(feature = "gpu")]
        let mut renderer = {
            // Block on async creation for simplicity in this synchronous run method
            // In a real app we might use a proper runtime like tokio
            futures::executor::block_on(GpuRenderer::new(window.clone()))
        };

        // Mount root (simulated)
        // runtime.mount(...);

        event_loop
            .run(move |event, target| {
                target.set_control_flow(ControlFlow::Poll);

                match event {
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        window_id,
                    } if window_id == window.id() => {
                        target.exit();
                    }
                    Event::WindowEvent {
                        event: WindowEvent::Resized(new_size),
                        window_id,
                    } if window_id == window.id() => {
                        #[cfg(feature = "gpu")]
                        renderer.resize(new_size);
                    }
                    Event::WindowEvent {
                        event: WindowEvent::RedrawRequested,
                        window_id,
                    } if window_id == window.id() => {
                        // Update Runtime
                        runtime.update();
                        let _mutations = runtime.drain_mutations();
                        // Process mutations (draw to verify, or update layout)

                        // Render
                        #[cfg(feature = "gpu")]
                        match renderer.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => renderer.resize(window.inner_size()),
                            Err(wgpu::SurfaceError::OutOfMemory) => target.exit(),
                            Err(e) => eprintln!("{:?}", e),
                        }
                    }
                    Event::AboutToWait => {
                        window.request_redraw();
                    }
                    _ => {}
                }
            })
            .unwrap();
    }
}
