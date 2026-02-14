use log::info;
use nexa_core::Runtime;
use nexa_renderer_gpu::GpuRenderer;
use std::sync::Mutex;

// Simple Thread-safe state container
pub struct MobileApp {
    pub runtime: Runtime,
    pub renderer: Option<GpuRenderer>,
    pub suspended: bool,
}

lazy_static::lazy_static! {
    pub static ref APP_INSTANCE: Mutex<Option<MobileApp>> = Mutex::new(None);
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
        if let Some(app) = APP_INSTANCE.lock().unwrap().as_mut() {
            app.suspended = false;
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn Java_com_nexa_NexaBridge_onPause(_env: JNIEnv, _class: JClass) {
        if let Some(app) = APP_INSTANCE.lock().unwrap().as_mut() {
            app.suspended = true;
        }
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
        let mut app = APP_INSTANCE.lock().unwrap();
        *app = Some(MobileApp {
            runtime: Runtime::new(),
            renderer: None,
            suspended: false,
        });
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn nexa_mobile_on_touch(x: f64, y: f64, phase: i32) {
        if let Some(_app) = APP_INSTANCE.lock().unwrap().as_mut() {
            // Translate touch to Nexa event
            info!("Touch event: ({}, {}) phase: {}", x, y, phase);
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn nexa_mobile_on_orientation_change(width: i32, height: i32) {
        if let Some(app) = APP_INSTANCE.lock().unwrap().as_mut() {
            if let Some(_r) = app.renderer.as_mut() {
                info!("Orientation change: {}x{}", width, height);
            }
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn nexa_mobile_update() {
        if let Some(app) = APP_INSTANCE.lock().unwrap().as_mut() {
            if !app.suspended {
                app.runtime.update();
                let _mutations = app.runtime.drain_mutations();
            }
        }
    }
}

// Default stub for host compilation check
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod host_stub {
    use super::*;
    pub fn verify() {
        let _ = MobileApp {
            runtime: Runtime::new(),
            renderer: None,
            suspended: false,
        };
    }
}
