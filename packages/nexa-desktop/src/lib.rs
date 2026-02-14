use arboard::Clipboard;
use log::{error, info};
use nexa_core::Runtime;
use nexa_renderer_gpu::{GpuRenderer, scene::Scene};
use rfd::FileDialog;
use std::sync::Arc;
use tray_icon::TrayIconBuilder;
use tray_icon::menu::{Menu, MenuItem};
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

pub struct DesktopApp {
    title: String,
    headless: bool,
}

pub struct IpcChannel {
    // Simple IPC abstraction
}

impl IpcChannel {
    pub fn send(&self, msg: &str) {
        info!("IPC Send: {}", msg);
    }
}

impl DesktopApp {
    pub fn new() -> Self {
        Self {
            title: "Nexa Desktop App".to_string(),
            headless: false,
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn with_headless(mut self, headless: bool) -> Self {
        self.headless = headless;
        self
    }

    pub fn run(self) {
        env_logger::init();
        info!("Starting Nexa Desktop...");

        let event_loop = EventLoop::new().unwrap();

        let window = if !self.headless {
            Some(Arc::new(
                WindowBuilder::new()
                    .with_title(&self.title)
                    .build(&event_loop)
                    .unwrap(),
            ))
        } else {
            None
        };

        // Initialize Native Components
        let mut clipboard = Clipboard::new().ok();
        let _tray = if !self.headless {
            let tray_menu = Menu::new();
            let _ = tray_menu.append(&MenuItem::new("Exit", true, None));

            Some(
                TrayIconBuilder::new()
                    .with_menu(Box::new(tray_menu))
                    .with_tooltip(&self.title)
                    .build()
                    .unwrap(),
            )
        } else {
            None
        };

        // Initialize Runtime
        let mut runtime = Runtime::new();
        let _ipc = IpcChannel {};

        // Initialize GPU Renderer
        let mut renderer = if let Some(ref win) = window {
            Some(futures::executor::block_on(GpuRenderer::new(win.clone())))
        } else {
            None
        };

        event_loop
            .run(move |event, target| {
                target.set_control_flow(ControlFlow::Poll);

                match event {
                    Event::WindowEvent {
                        event: win_event,
                        window_id,
                    } => {
                        if let Some(ref win) = window {
                            if window_id == win.id() {
                                match win_event {
                                    WindowEvent::CloseRequested => {
                                        info!("Close requested, shutting down gracefully...");
                                        target.exit();
                                    }
                                    WindowEvent::Resized(new_size) => {
                                        if let Some(ref mut r) = renderer {
                                            r.resize(new_size);
                                        }
                                    }
                                    WindowEvent::Focused(focused) => {
                                        info!("Window focused: {}", focused);
                                    }
                                    WindowEvent::KeyboardInput {
                                        event:
                                            KeyEvent {
                                                physical_key: PhysicalKey::Code(code),
                                                state: ElementState::Pressed,
                                                ..
                                            },
                                        ..
                                    } => {
                                        // Handle keyboard shortcuts
                                        match code {
                                            KeyCode::KeyO => {
                                                let files = FileDialog::new()
                                                    .set_directory("/")
                                                    .pick_file();
                                                info!("File picked: {:?}", files);
                                            }
                                            KeyCode::KeyC => {
                                                if let Some(ref mut cb) = clipboard {
                                                    let _ = cb.set_text("Nexa Clipboard Content");
                                                    info!("Text copied to clipboard");
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    WindowEvent::RedrawRequested => {
                                        runtime.update();
                                        let _mutations = runtime.drain_mutations();

                                        if let Some(ref mut r) = renderer {
                                            let mut scene = Scene {
                                                root: nexa_renderer_gpu::SceneNode::Container {
                                                    transform: glam::Mat4::IDENTITY,
                                                    children: vec![],
                                                    is_dirty: true,
                                                },
                                                last_frame_time: std::time::Duration::from_secs(0),
                                            };

                                            match r.render(&mut scene) {
                                                Ok(_) => {}
                                                Err(wgpu::SurfaceError::Lost) => {
                                                    r.resize(win.inner_size())
                                                }
                                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                                    target.exit()
                                                }
                                                Err(e) => error!("Render error: {:?}", e),
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    Event::AboutToWait => {
                        if let Some(ref win) = window {
                            win.request_redraw();
                        } else if self.headless {
                            // In headless mode, we still want to poll runtime
                            runtime.update();
                            let _mutations = runtime.drain_mutations();
                            // Optional: Sleep or break loop for testing
                        }
                    }
                    _ => {}
                }
            })
            .unwrap();
    }
}
