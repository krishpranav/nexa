use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use nexa_renderer_gpu::{GpuRenderer, SceneNode};
use std::sync::Arc;

pub fn launch<F>(app: F)
where
    F: Fn() -> SceneNode + 'static,
{
    // Mobile launch logic is very similar to desktop due to Winit abstraction, 
    // but Android/iOS have specific lifecycle events (Suspend/Resume).
    // For scaffolding, we use a basic loop.

    let event_loop = EventLoop::new().unwrap();
    let window = Arc::new(WindowBuilder::new().title("Nexa Mobile").build(&event_loop).unwrap());
    
    // On mobile we don't start a tokio runtime in the same way usually, 
    // but for wgpu initialization we need async executor.
    // We'll assume some platform executor or block_on for init.
    // NOTE: On Android, blocking main thread is bad, but for init it might be okay or offloaded.
    
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let mut renderer = runtime.block_on(async {
        GpuRenderer::new(window.clone()).await
    });

    event_loop.run(move |event, target| {
        target.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                target.exit();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(physical_size),
                ..
            } => {
                renderer.resize(physical_size);
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                let scene = app();
                let _ = renderer.render(&scene);
            }
            Event::Resumed => {
                // Mobile specific resume logic: recreate surface if lost?
                // winit handles some of this.
            }
            Event::Suspended => {
                // Handle suspend
            }
            Event::AboutToWait => {
                 window.request_redraw();
            }
            _ => {}
        }
    }).unwrap();
}
