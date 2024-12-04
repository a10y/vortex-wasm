mod utils;

use crate::utils::{set_panic_hook, BlobReader};
use bytes::BytesMut;
use futures_util::StreamExt;
use std::convert::Into;
use std::sync::Arc;
use vortex::array::ChunkedArray;
use vortex::compute::scalar_at;
use vortex::dtype::{DType, PType};
use vortex::file::{LayoutContext, LayoutDeserializer, VortexReadBuilder};
use vortex::sampling_compressor::ALL_ENCODINGS_CONTEXT;
use vortex::scalar::Scalar;
use vortex::{ArrayData, IntoArrayData};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::{Map, Object, Uint8Array};
use web_sys::{
    Blob, ReadableStreamByobReader, ReadableStreamGetReaderOptions, ReadableStreamReadResult,
    ReadableStreamReaderMode,
};

#[wasm_bindgen(js_name = File)]
pub struct VortexFile {
    reader: BlobReader,
}

#[wasm_bindgen(start)]
fn start() {
    web_sys::console::log_1(&"vortex-wasm starting".into());
    web_sys::console::log_1(&"setting panic hook...".into());
    set_panic_hook();
}

#[wasm_bindgen(js_class = File)]
impl VortexFile {
    #[wasm_bindgen(js_name = fromBlob)]
    pub async fn from_blob(blob: Blob) -> Self {
        Self {
            reader: BlobReader(Arc::new(blob)),
        }
    }

    /// Read from a blob into the allocation at the provided base address.
    #[wasm_bindgen(js_name = fromBlobOld)]
    pub async fn from_blob_old(blob: Blob) -> Self {
        // let blob = value.dyn_into::<Blob>().expect("expected a blob");
        let len = blob.size() as u32;
        web_sys::console::log_1(&format!("blob size = {}", blob.size()).into());
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
        // Self {
        //     buffer: target.freeze(),
        // }
        todo!()
    }

    /// Log the DType to the console.
    #[wasm_bindgen(js_name = printSchema)]
    pub async fn print_schema(&self) {
        // let buffer = self.buffer.clone();
        let inner = self.reader.clone();
        let reader = VortexReadBuilder::new(
            inner,
            LayoutDeserializer::new(
                ALL_ENCODINGS_CONTEXT.clone(),
                LayoutContext::default().into(),
            ),
        )
        .build()
        .await
        .expect("building reader");

        web_sys::console::log_1(&format!("dtype = {}", reader.dtype()).into());
        web_sys::console::log_1(&format!("row_count = {}", reader.row_count()).into());
    }

    /// Materialize the entire array.
    #[wasm_bindgen]
    pub async fn collect(&self) -> Array {
        let mut reader = VortexReadBuilder::new(
            self.reader.clone(),
            // self.buffer.clone(),
            LayoutDeserializer::new(
                ALL_ENCODINGS_CONTEXT.clone(),
                LayoutContext::default().into(),
            ),
        )
        .build()
        .await
        .expect("building reader");

        let dtype = reader.dtype().clone();
        let mut chunks = Vec::new();
        while let Some(next) = reader.next().await {
            let next = next.unwrap();
            web_sys::console::log_1(&format!("loaded another chunk if len {}", next.len()).into());
            chunks.push(next);
        }

        let chunked = ChunkedArray::try_new(chunks, dtype).unwrap().into_array();

        Array { inner: chunked }
    }
}

#[wasm_bindgen]
pub struct Array {
    inner: ArrayData,
}

#[wasm_bindgen]
impl Array {
    #[wasm_bindgen]
    pub fn get(&self, index: u32) -> JsValue {
        let scalar = scalar_at(&self.inner, index as usize).unwrap();
        to_js_val(scalar)
    }
}

fn to_js_val(scalar: Scalar) -> JsValue {
    match scalar.dtype() {
        DType::Null => JsValue::null(),
        DType::Bool(_) => scalar
            .as_bool()
            .value()
            .map(JsValue::from_bool)
            .unwrap_or_else(|| JsValue::null()),
        DType::Primitive(ptype, _) => {
            // The scalar needs to be up-cast to f64 because that is all
            // JavaScript can represent.
            let maybe_f64_scalar = match ptype {
                PType::U8 => scalar.as_primitive().typed_value::<u8>().map(JsValue::from),
                PType::U16 => scalar
                    .as_primitive()
                    .typed_value::<u16>()
                    .map(JsValue::from),
                PType::U32 => scalar
                    .as_primitive()
                    .typed_value::<u32>()
                    .map(JsValue::from),
                PType::U64 => scalar
                    .as_primitive()
                    .typed_value::<u64>()
                    .map(JsValue::from),
                PType::I8 => scalar.as_primitive().typed_value::<i8>().map(JsValue::from),
                PType::I16 => scalar
                    .as_primitive()
                    .typed_value::<i16>()
                    .map(JsValue::from),
                PType::I32 => scalar
                    .as_primitive()
                    .typed_value::<i32>()
                    .map(JsValue::from),
                PType::I64 => scalar
                    .as_primitive()
                    .typed_value::<i64>()
                    .map(JsValue::from),
                PType::F16 => {
                    panic!("invalid type");
                }
                PType::F32 => scalar
                    .as_primitive()
                    .typed_value::<f32>()
                    .map(JsValue::from),
                PType::F64 => scalar
                    .as_primitive()
                    .typed_value::<f64>()
                    .map(JsValue::from),
            };

            // fallback to null
            maybe_f64_scalar.unwrap_or_else(|| JsValue::null())
        }
        DType::Utf8(_) => scalar
            .as_utf8()
            .value()
            .map(|string| JsValue::from_str(string.as_str()))
            .unwrap_or_else(|| JsValue::null()),
        DType::Binary(_) => {
            scalar
                .as_binary()
                .value()
                .map(|binary| {
                    // Copy the data into the Uint8Array.
                    let buffer = Uint8Array::new_with_length(binary.len() as u32);
                    buffer.copy_from(binary.as_slice());
                    JsValue::from(buffer)
                })
                .unwrap_or_else(|| JsValue::null())
        }
        DType::Struct(_, _) => {
            // recursively generate the struct
            let struct_scalar = scalar.as_struct();
            let field_names = struct_scalar.dtype().as_struct().unwrap().names().clone();
            let Some(fields) = struct_scalar.fields() else {
                return JsValue::null();
            };

            // Create a new JS Object to hold all of the fields.
            let properties = Map::new();
            for (field_name, scalar) in field_names.into_iter().zip(fields.into_iter()) {
                properties.set(&field_name.to_string().into(), &to_js_val(scalar));
            }

            // Freeze the object
            let js_obj = Object::from_entries(properties.as_ref()).unwrap();
            Object::freeze(&js_obj).into()
        }
        DType::List(_, _) => {
            panic!("lol");
        }
        DType::Extension(_) => JsValue::from_str("fix handling of ExtensionDType"),
    }
}
