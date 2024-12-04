mod utils;

use crate::utils::set_panic_hook;
use bytes::{Bytes, BytesMut};
use std::convert::Into;
use vortex::file::{LayoutContext, LayoutDeserializer, VortexReadBuilder};
use vortex::sampling_compressor::ALL_ENCODINGS_CONTEXT;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::Uint8Array;
use web_sys::{
    Blob, ReadableStreamByobReader, ReadableStreamGetReaderOptions, ReadableStreamReadResult,
    ReadableStreamReaderMode,
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);
}

#[wasm_bindgen]
pub struct VortexFile {
    buffer: Bytes,
}

#[wasm_bindgen(start)]
fn start() {
    log("vortex-wasm starting");
    log("setting panic hook...");
    set_panic_hook();

    // log("attempting to grow memory by 100 pages...");
    // let result = wasm_bindgen::memory()
    //     .unchecked_into::<WebAssembly::Memory>()
    //     .grow(4_000);
    // if result == u32::MAX {
    //     error("memory grow failed");
    // } else {
    //     log("memory grow succeeded");
    // }
}

#[wasm_bindgen]
impl VortexFile {
    /// Read from a blob into the allocation at the provided base address.
    #[wasm_bindgen(js_name = fromBlob)]
    pub async fn from_blob(value: JsValue) -> Self {
        let blob = value.dyn_into::<Blob>().expect("expected a blob");
        let len = blob.size() as u32;
        log(&format!("blob size = {}", blob.size()));
        let reader_opts = ReadableStreamGetReaderOptions::new();
        reader_opts.set_mode(ReadableStreamReaderMode::Byob);

        // Give it access to an ArrayBuffer that points to local WASM memory.
        let stream_reader = blob
            .stream()
            .get_reader_with_options(&reader_opts)
            .dyn_into::<ReadableStreamByobReader>()
            .expect("get BYOB reader");

        // Pre-allocate the Rust memory we land data into.
        // Sadly there is not a way to truly do this with zero copy.
        let mut target = BytesMut::with_capacity(len as usize);
        unsafe {
            target.set_len(len as usize);
        }

        // Pre-allocate a memory buffer
        let mut mem = Uint8Array::new_with_length(len);
        let mut offset = 0;
        while offset < len {
            let result = JsFuture::from(stream_reader.read_with_array_buffer_view(mem.as_ref()))
                .await
                .unwrap();

            let result = ReadableStreamReadResult::from(result);

            let value = result.get_value();
            let done = result.get_done().unwrap_or_default();
            if value.is_undefined() {
                assert!(done, "value undefined before done");
            }

            let value = value.unchecked_into::<Uint8Array>();
            let target_start = offset as usize;
            let target_end = (offset + value.byte_length()) as usize;
            value.copy_to(&mut target[target_start..target_end]);

            offset += value.byte_length();

            mem = value;
        }

        // The data has been written to the byte range directly.
        // We have the buffer values available immediately.
        // We can access the JS-side reference to our memory.
        Self {
            buffer: target.freeze(),
        }
    }

    /// Log the DType to the console.
    #[wasm_bindgen(js_name = printSchema)]
    pub async fn print_schema(&self) {
        let buffer = self.buffer.clone();
        let reader = VortexReadBuilder::new(
            buffer,
            LayoutDeserializer::new(
                ALL_ENCODINGS_CONTEXT.clone(),
                LayoutContext::default().into(),
            ),
        )
        .build()
        .await
        .expect("building reader");

        log(format!("dtype = {}", reader.dtype()).as_str());
        log(format!("row_count = {}", reader.row_count()).as_str());
    }
}
