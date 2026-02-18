use log::info;
use nexa_core::Runtime;
use nexa_renderer_gpu::GpuRenderer;
use nexa_scheduler::Scheduler;
use std::cell::RefCell;

// Simple Thread-safe state container
pub struct MobileApp {
    pub runtime: Runtime<Scheduler>,
    pub renderer: Option<GpuRenderer>,
    pub suspended: bool,
}

thread_local! {
    pub static APP_INSTANCE: RefCell<Option<MobileApp>> = RefCell::new(None);
}

// Resource loading abstraction
pub trait ResourceLoader {
    fn load_asset(&self, path: &str) -> Vec<u8>;
}

// --- Android Bindings ---
#[cfg(target_os = "android")]
pub mod android {
    use super::*;
    use jni::JNIEnv;
    use jni::objects::JClass;
    use winit::platform::android::activity::AndroidApp;

    #[unsafe(no_mangle)]
    pub extern "C" fn android_main(app: AndroidApp) {
        info!("Nexa Android Entry Point");
        // In a real app, this would build the Winit EventLoop
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn Java_com_nexa_NexaBridge_onStart(_env: JNIEnv, _class: JClass) {
        info!("Android onStart");
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn Java_com_nexa_NexaBridge_onResume(_env: JNIEnv, _class: JClass) {
        APP_INSTANCE.with(|app| {
            if let Some(app) = app.borrow_mut().as_mut() {
                app.suspended = false;
            }
        });
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn Java_com_nexa_NexaBridge_onPause(_env: JNIEnv, _class: JClass) {
        APP_INSTANCE.with(|app| {
            if let Some(app) = app.borrow_mut().as_mut() {
                app.suspended = true;
            }
        });
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn Java_com_nexa_NexaBridge_onLowMemory(_env: JNIEnv, _class: JClass) {
        warn!("Mobile: Low Memory Pressure detected!");
        // Clear caches
    }
}

// --- iOS / Swift C Bindings ---
pub mod ios {
    use super::*;

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn nexa_mobile_init() {
        APP_INSTANCE.with(|app| {
            *app.borrow_mut() = Some(MobileApp {
                runtime: Runtime::new(Scheduler::new()),
                renderer: None,
                suspended: false,
            });
        });
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn nexa_mobile_on_touch(x: f64, y: f64, phase: i32) {
        APP_INSTANCE.with(|app| {
            if let Some(_app) = app.borrow_mut().as_mut() {
                // Translate touch to Nexa event
                info!("Touch event: ({}, {}) phase: {}", x, y, phase);
            }
        });
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn nexa_mobile_on_orientation_change(width: i32, height: i32) {
        APP_INSTANCE.with(|app| {
            if let Some(app) = app.borrow_mut().as_mut() {
                if let Some(_r) = app.renderer.as_mut() {
                    info!("Orientation change: {}x{}", width, height);
                }
            }
        });
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn nexa_mobile_update() {
        APP_INSTANCE.with(|app| {
            if let Some(app) = app.borrow_mut().as_mut() {
                if !app.suspended {
                    app.runtime.update();
                    let _mutations = app.runtime.drain_mutations();
                }
            }
        });
    }
}

// Default stub for host compilation check
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod host_stub {
    use super::*;
    pub fn verify() {
        let _ = MobileApp {
            runtime: Runtime::new(Scheduler::new()),
            renderer: None,
            suspended: false,
        };
    }
}
