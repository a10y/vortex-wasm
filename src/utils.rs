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

// pub struct BlobReader(Blob);
//
// impl VortexReadAt for BlobReader {
//     fn read_byte_range(&self, pos: u64, len: u64) -> impl Future<Output=std::io::Result<Bytes>> + 'static {
//         let this = self.clone();
//         // Perform a direct read against a blob
//         spawn_local(async move {
//             let start: i32 = pos.try_into().unwrap();
//             let end: i32 = (pos + len).try_into().unwrap();
//             let sliced = this.0.slice_with_i32_and_i32(start, end).unwrap();
//             let stream = ReadableStreamDefaultReader::new(&sliced.stream()).unwrap();
//             let result: ReadableStreamReadResult = JsFuture::from(stream.read()).await.unwrap().unchecked_into();
//         })
//     }
//
//     fn size(&self) -> impl Future<Output=std::io::Result<u64>> + 'static {
//         todo!()
//     }
// }
