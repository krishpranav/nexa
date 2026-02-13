use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use nexa_renderer-gpu::{GpuRenderer, SceneNode};
use std::sync::Arc;

pub fn launch<F>(app: F)
where
    F: Fn() -> SceneNode + 'static, // Simplified for now
{
    let event_loop = EventLoop::new().unwrap();
    let window = Arc::new(WindowBuilder::new().title("Nexa Desktop").build(&event_loop).unwrap());

    // We need async for GPU init, so block_on or spawn
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
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
                match renderer.render(&scene) {
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
    }).unwrap();
}
