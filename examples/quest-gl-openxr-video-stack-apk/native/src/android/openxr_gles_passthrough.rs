use std::ptr;

use openxr as xr;
use openxr::sys::Handle as _;

use super::{ensure_xr_success, log_error, log_info};

pub(super) struct OpenXrGlesPassthroughUnderlay {
    fb_passthrough: xr::raw::PassthroughFB,
    passthrough: xr::sys::PassthroughFB,
    pub(super) layer: xr::sys::PassthroughLayerFB,
}

impl Drop for OpenXrGlesPassthroughUnderlay {
    fn drop(&mut self) {
        unsafe {
            if self.layer != xr::sys::PassthroughLayerFB::NULL {
                let pause_result = (self.fb_passthrough.passthrough_layer_pause)(self.layer);
                if pause_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                    log_error(format!(
                        "Rusty XR OpenXR GLES passthrough layer pause during drop failed result={pause_result:?}"
                    ));
                }
                let destroy_result = (self.fb_passthrough.destroy_passthrough_layer)(self.layer);
                if destroy_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                    log_error(format!(
                        "Rusty XR OpenXR GLES passthrough layer destroy failed result={destroy_result:?}"
                    ));
                }
                self.layer = xr::sys::PassthroughLayerFB::NULL;
            }
            if self.passthrough != xr::sys::PassthroughFB::NULL {
                let pause_result = (self.fb_passthrough.passthrough_pause)(self.passthrough);
                if pause_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                    log_error(format!(
                        "Rusty XR OpenXR GLES passthrough pause during drop failed result={pause_result:?}"
                    ));
                }
                let destroy_result = (self.fb_passthrough.destroy_passthrough)(self.passthrough);
                if destroy_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                    log_error(format!(
                        "Rusty XR OpenXR GLES passthrough destroy failed result={destroy_result:?}"
                    ));
                }
                self.passthrough = xr::sys::PassthroughFB::NULL;
            }
        }
    }
}

pub(super) fn create_openxr_gles_passthrough_underlay(
    instance: &xr::Instance,
    session: &xr::Session<xr::OpenGlEs>,
) -> Result<OpenXrGlesPassthroughUnderlay, String> {
    let fb_passthrough = *instance
        .exts()
        .fb_passthrough
        .as_ref()
        .ok_or_else(|| "XR_FB_passthrough function table is unavailable".to_string())?;
    let flags = xr::PassthroughFlagsFB::EMPTY;
    let passthrough_info = xr::sys::PassthroughCreateInfoFB {
        ty: xr::sys::PassthroughCreateInfoFB::TYPE,
        next: ptr::null(),
        flags,
    };
    let mut passthrough = xr::sys::PassthroughFB::NULL;
    let result = unsafe {
        (fb_passthrough.create_passthrough)(session.as_raw(), &passthrough_info, &mut passthrough)
    };
    ensure_xr_success(result, "xrCreatePassthroughFB")?;

    let layer_info = xr::sys::PassthroughLayerCreateInfoFB {
        ty: xr::sys::PassthroughLayerCreateInfoFB::TYPE,
        next: ptr::null(),
        passthrough,
        flags,
        purpose: xr::PassthroughLayerPurposeFB::RECONSTRUCTION,
    };
    let mut layer = xr::sys::PassthroughLayerFB::NULL;
    let result = unsafe {
        (fb_passthrough.create_passthrough_layer)(session.as_raw(), &layer_info, &mut layer)
    };
    if let Err(error) = ensure_xr_success(result, "xrCreatePassthroughLayerFB") {
        let destroy_result = unsafe { (fb_passthrough.destroy_passthrough)(passthrough) };
        if destroy_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
            log_error(format!(
                "Rusty XR OpenXR GLES passthrough cleanup after layer create failed result={destroy_result:?}"
            ));
        }
        return Err(error);
    }

    let result = unsafe { (fb_passthrough.passthrough_start)(passthrough) };
    if let Err(error) = ensure_xr_success(result, "xrPassthroughStartFB") {
        unsafe {
            let layer_destroy_result = (fb_passthrough.destroy_passthrough_layer)(layer);
            if layer_destroy_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR OpenXR GLES passthrough layer cleanup after start failed result={layer_destroy_result:?}"
                ));
            }
            let passthrough_destroy_result = (fb_passthrough.destroy_passthrough)(passthrough);
            if passthrough_destroy_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR OpenXR GLES passthrough cleanup after start failed result={passthrough_destroy_result:?}"
                ));
            }
        }
        return Err(error);
    }

    let result = unsafe { (fb_passthrough.passthrough_layer_resume)(layer) };
    if let Err(error) = ensure_xr_success(result, "xrPassthroughLayerResumeFB") {
        unsafe {
            let passthrough_pause_result = (fb_passthrough.passthrough_pause)(passthrough);
            if passthrough_pause_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR OpenXR GLES passthrough pause cleanup after layer resume failed result={passthrough_pause_result:?}"
                ));
            }
            let layer_destroy_result = (fb_passthrough.destroy_passthrough_layer)(layer);
            if layer_destroy_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR OpenXR GLES passthrough layer cleanup after resume failed result={layer_destroy_result:?}"
                ));
            }
            let passthrough_destroy_result = (fb_passthrough.destroy_passthrough)(passthrough);
            if passthrough_destroy_result.into_raw() < xr::sys::Result::SUCCESS.into_raw() {
                log_error(format!(
                    "Rusty XR OpenXR GLES passthrough cleanup after resume failed result={passthrough_destroy_result:?}"
                ));
            }
        }
        return Err(error);
    }

    log_info(format!(
        "Rusty XR OpenXR GLES passthrough started purpose={:?}",
        xr::PassthroughLayerPurposeFB::RECONSTRUCTION
    ));

    Ok(OpenXrGlesPassthroughUnderlay {
        fb_passthrough,
        passthrough,
        layer,
    })
}
