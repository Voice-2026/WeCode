use crate::common::set_last_error;
use jni::{
    JNIEnv,
    objects::{GlobalRef, JClass, JObject},
    sys::{JNI_FALSE, JNI_TRUE, jboolean},
};
use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Mutex, OnceLock};

static ANDROID_APPLICATION_CONTEXT: OnceLock<GlobalRef> = OnceLock::new();
static ANDROID_CONTEXT_INIT_LOCK: Mutex<()> = Mutex::new(());

#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_codux_codux_1protocol_1ffi_CoduxProtocolFfiPlugin_initAndroidContext(
    mut env: JNIEnv,
    _class: JClass,
    context: JObject,
) -> jboolean {
    match catch_unwind(AssertUnwindSafe(|| init_android_context(&mut env, context))) {
        Ok(Ok(())) => JNI_TRUE,
        Ok(Err(error)) => {
            set_last_error(error);
            JNI_FALSE
        }
        Err(_) => {
            set_last_error("Android JNI context initialization panicked");
            JNI_FALSE
        }
    }
}

fn init_android_context(env: &mut JNIEnv, context: JObject) -> Result<(), String> {
    let _guard = ANDROID_CONTEXT_INIT_LOCK
        .lock()
        .map_err(|_| "Android JNI context init lock was poisoned".to_string())?;
    if ANDROID_APPLICATION_CONTEXT.get().is_some() {
        return Ok(());
    }
    let application_context = env
        .call_method(
            context,
            "getApplicationContext",
            "()Landroid/content/Context;",
            &[],
        )
        .map_err(|error| format!("failed to read Android application context: {error}"))?
        .l()
        .map_err(|error| format!("invalid Android application context: {error}"))?;
    let application_context = env
        .new_global_ref(application_context)
        .map_err(|error| format!("failed to pin Android application context: {error}"))?;
    let java_vm = env
        .get_java_vm()
        .map_err(|error| format!("failed to read Android JavaVM: {error}"))?;
    let java_vm = java_vm.get_java_vm_pointer().cast::<c_void>();
    let application_context_ptr = application_context.as_obj().as_raw().cast::<c_void>();
    unsafe {
        codux_remote_transport::install_android_jni_context(java_vm, application_context_ptr)?;
    }
    let _ = ANDROID_APPLICATION_CONTEXT.set(application_context);
    Ok(())
}
