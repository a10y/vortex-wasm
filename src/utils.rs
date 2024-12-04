use bytes::{Bytes, BytesMut};
use futures_channel::oneshot;
use futures_util::FutureExt;
use js_sys::Uint8Array;
use std::convert::TryInto;
use std::future::Future;
use std::sync::Arc;
use vortex::io::VortexReadAt;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Blob, FileReader};

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

#[derive(Clone)]
pub struct BlobReader(pub Arc<Blob>);

// We know this is ok b/c web is single-threaded.
unsafe impl Send for BlobReader {}
unsafe impl Sync for BlobReader {}

impl VortexReadAt for BlobReader {
    fn read_byte_range(
        &self,
        pos: u64,
        len: u64,
    ) -> impl Future<Output = std::io::Result<Bytes>> + 'static {
        let this = self.clone();
        web_sys::console::log_1(&format!("read_byte_range({pos}, {len})").into());

        let (tx, rx) = oneshot::channel();

        let start: i32 = pos.try_into().unwrap();
        let end: i32 = (pos + len).try_into().unwrap();
        let sliced = this.0.slice_with_i32_and_i32(start, end).unwrap();

        let file_reader = FileReader::new().unwrap();
        let file_reader_cb = file_reader.clone();

        // Send the onload handler
        let loadend = Closure::once_into_js(move || {
            let array_buf = file_reader_cb.result().unwrap();
            let array = Uint8Array::new(array_buf.as_ref());
            let mut result = BytesMut::with_capacity(len.try_into().unwrap());
            unsafe {
                result.set_len(result.capacity());
            }
            array.copy_to(&mut result);

            // Send the result to the main thread.
            tx.send(result).unwrap();
        });
        file_reader.set_onloadend(loadend.dyn_ref());

        // Trigger the streaming read.
        file_reader.read_as_array_buffer(&sliced).unwrap();

        // Return the reader which will be awaited.
        rx.map(|res| Ok(res.unwrap().freeze()))
    }

    fn size(&self) -> impl Future<Output = std::io::Result<u64>> + 'static {
        std::future::ready(Ok(self.0.size() as u64))
    }
}
