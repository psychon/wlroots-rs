//! Manager that is called when an output is created or destroyed.
//! Pass a struct that implements this trait to the `Compositor` during
//! initialization.


use libc;
use manager::{OutputHandler, UserOutput};
use types::OutputHandle;
use wlroots_sys::wlr_output;

use wayland_sys::server::WAYLAND_SERVER_HANDLE;
use wayland_sys::server::signal::wl_signal_add;

pub struct OutputBuilder<'output> {
    output: &'output mut OutputHandle
}

/// Used to ensure that the builder is used to construct
/// the OutputHandler instance.
pub struct OutputBuilderResult<'output> {
    pub output: &'output mut OutputHandle,
    result: Box<OutputHandler>
}

impl<'output> OutputBuilder<'output> {
    pub fn build_best_mode<T: OutputHandler + 'static>(self,
                                                       data: T)
                                                       -> OutputBuilderResult<'output> {
        self.output.choose_best_mode();
        OutputBuilderResult {
            output: self.output,
            result: Box::new(data)
        }
    }
}

/// Handles output addition and removal.
pub trait OutputManagerHandler {
    /// Called whenever an output is added.
    fn output_added<'output>(&mut self,
                             _: OutputBuilder<'output>)
                             -> Option<OutputBuilderResult<'output>> {
        None
    }

    /// Called whenever an output is removed.
    fn output_removed(&mut self, &mut OutputHandle) {
        // TODO
    }
    /// Called every time the output frame is updated.
    fn output_frame(&mut self, &mut OutputHandle) {}
    /// Called every time the output resolution is updated.
    fn output_resolution(&mut self, &mut OutputHandle) {}
}

wayland_listener!(OutputManager, (Vec<Box<UserOutput>>, Box<OutputManagerHandler>), [
    add_listener => add_notify: |this: &mut OutputManager, data: *mut libc::c_void,| unsafe {
        let (ref mut outputs, ref mut manager) = this.data;
        let data = data as *mut wlr_output;
        let mut output = OutputHandle::from_ptr(data as *mut wlr_output);
        let builder = OutputBuilder { output: &mut output };
        if let Some(OutputBuilderResult {result: output, ..}) = manager.output_added(builder) {
            let mut output = UserOutput::new((data, output));
            // Add the output frame event to this manager
            wl_signal_add(&mut (*data).events.frame as *mut _ as _,
                        output.frame_listener() as _);
            // Add the output resolution event to this manager
            wl_signal_add(&mut (*data).events.resolution as *mut _ as _,
                        output.resolution_listener() as _);
            // Store the user UserOutput, free later in remove listener
            outputs.push(output);
        }
    };
    remove_listener => remove_notify: |this: &mut OutputManager, data: *mut libc::c_void,| unsafe {
        let (ref mut outputs, ref mut manager) = this.data;
        let data = data as *mut wlr_output;
        let mut output = OutputHandle::from_ptr(data);
        manager.output_removed(&mut output);
        if let Some(layout) = output.layout() {
            layout.borrow_mut().remove(&mut output);
        }
        // Remove user output data
        if let Some(index) = outputs.iter().position(|output| output.output_ptr() == data) {
            let mut removed_output = outputs.remove(index);
            ffi_dispatch!(WAYLAND_SERVER_HANDLE,
                          wl_list_remove,
                          &mut (*removed_output.frame_listener()).link as *mut _ as _);
            ffi_dispatch!(WAYLAND_SERVER_HANDLE,
                          wl_list_remove,
                          &mut (*removed_output.resolution_listener()).link as *mut _ as _);

        }
    };
]);
