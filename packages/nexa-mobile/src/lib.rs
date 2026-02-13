use nexa_core::Runtime;
use nexa_renderer_gpu::GpuRenderer;
use std::sync::Arc;
// use winit::... we need to hook into the platform's window

// Shared state container
struct MobileApp {
    runtime: Runtime,
    renderer: Option<GpuRenderer>,
}

static mut APP_INSTANCE: Option<MobileApp> = None;

// --- Android JNI Bindings ---
#[cfg(target_os = "android")]
#[allow(non_snake_case)]
pub mod android {
    use super::*;
    use jni::objects::JClass;
    use jni::sys::jint;
    use jni::JNIEnv;
    use winit::platform::android::activity::AndroidApp;

    #[no_mangle]
    pub extern "C" fn android_main(app: AndroidApp) {
        // This is the entry point for `android-activity`.
        // In a real winit app on Android, we'd pass this `app` to winit's EventLoop builder.
        // For this minimal binding stub, we just verify compilation of JNI deps.
    }

    // Example JNI call to init
    #[no_mangle]
    pub unsafe extern "C" fn Java_com_nexa_NexaBridge_init(env: JNIEnv, _class: JClass) {
        // Initialize runtime
        APP_INSTANCE = Some(MobileApp {
            runtime: Runtime::new(),
            renderer: None, // Initialized later when surface is ready
        });
    }
}

// --- iOS / Swift C Bindings ---
#[cfg(target_os = "ios")]
pub mod ios {
    use super::*;
    use std::ffi::c_void;

    #[no_mangle]
    pub unsafe extern "C" fn nexa_mobile_init() {
        APP_INSTANCE = Some(MobileApp {
            runtime: Runtime::new(),
            renderer: None,
        });
    }

    #[no_mangle]
    pub unsafe extern "C" fn nexa_mobile_update() {
        if let Some(app) = APP_INSTANCE.as_mut() {
            app.runtime.update();
            // TODO: Process mutations
        }
    }

    // Stub to pass raw window handle pointer from Swift (UIView layer)
    #[no_mangle]
    pub unsafe extern "C" fn nexa_mobile_set_surface(view_ptr: *mut c_void) {
        // Conversion logic from raw pointer to RawWindowHandle would happen here
        // creating a surface for wgpu.
        // Since we don't have the full iOS toolchain context here, we just stub it.
    }
}

// Default stub for host compilation check
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod host_stub {
    use super::*;

    pub fn init() {
        // Just used to verify struct composition compiles
        let _ = MobileApp {
            runtime: Runtime::new(),
            renderer: None,
        };
    }
}
