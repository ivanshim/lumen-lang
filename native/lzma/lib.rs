//! One-shot decoding through the platform liblzma ABI.
use std::ffi::c_void;

#[repr(C)]
struct Stream {
    next_in: *const u8,
    avail_in: usize,
    total_in: u64,
    next_out: *mut u8,
    avail_out: usize,
    total_out: u64,
    allocator: *const c_void,
    internal: *mut c_void,
    reserved_ptrs: [*mut c_void; 4],
    reserved_u64: [u64; 2],
    reserved_size: [usize; 2],
    reserved_enum: [i32; 2],
}

#[link(name = "lzma")]
extern "C" {
    fn lzma_auto_decoder(stream: *mut Stream, limit: u64, flags: u32) -> i32;
    fn lzma_stream_decoder(stream: *mut Stream, limit: u64, flags: u32) -> i32;
    fn lzma_alone_decoder(stream: *mut Stream, limit: u64) -> i32;
    fn lzma_code(stream: *mut Stream, action: i32) -> i32;
    fn lzma_end(stream: *mut Stream);
    fn lzma_stream_buffer_bound(size: usize) -> usize;
    fn lzma_easy_buffer_encode(preset: u32, check: i32, allocator: *const c_void,
        input: *const u8, input_size: usize, output: *mut u8, output_pos: *mut usize,
        output_size: usize) -> i32;
}

impl Drop for Stream {
    // Release the decoder state on success and every error path.
    fn drop(&mut self) { unsafe { lzma_end(self); } }
}

// Decode one stream and retain its consumed length for concatenated input.
pub fn decode(input: &[u8], format: i64, limit: u64) -> Result<(Vec<u8>, usize), i32> {
    let mut state: Stream = unsafe { std::mem::zeroed() };
    let status = unsafe {
        match format {
            0 => lzma_auto_decoder(&mut state, limit, 0),
            1 => lzma_stream_decoder(&mut state, limit, 0),
            2 => lzma_alone_decoder(&mut state, limit),
            _ => return Err(8),
        }
    };
    if status != 0 { return Err(status); }
    state.next_in = input.as_ptr();
    state.avail_in = input.len();
    let mut output = Vec::new();
    loop {
        let mut chunk = [0u8; 16384];
        state.next_out = chunk.as_mut_ptr();
        state.avail_out = chunk.len();
        let status = unsafe { lzma_code(&mut state, 3) };
        let count = chunk.len() - state.avail_out;
        output.try_reserve(count).map_err(|_| 5)?;
        output.extend_from_slice(&chunk[..count]);
        match status {
            0 => (),
            1 => return Ok((output, input.len() - state.avail_in)),
            fault => return Err(fault),
        }
    }
}

// Encode an XZ stream with the platform codec's validated preset and check.
pub fn encode(input: &[u8], preset: u32, check: i32) -> Result<Vec<u8>, i32> {
    let capacity = unsafe { lzma_stream_buffer_bound(input.len()) };
    let mut output = Vec::new();
    output.try_reserve_exact(capacity).map_err(|_| 5)?;
    output.resize(capacity, 0);
    let mut used = 0;
    let status = unsafe { lzma_easy_buffer_encode(preset, check, std::ptr::null(),
        input.as_ptr(), input.len(), output.as_mut_ptr(), &mut used, capacity) };
    if status != 0 { return Err(status); }
    output.truncate(used);
    Ok(output)
}

#[cfg(test)]
mod tests {
    // Round-trip real compressed data through the platform ABI.
    #[test]
    fn xz_roundtrip() {
        let raw = b"lumen compressed archive";
        let encoded = super::encode(raw, 6, 4).unwrap();
        let (decoded, used) = super::decode(&encoded, 1, u64::MAX).unwrap();
        assert_eq!(decoded, raw);
        assert_eq!(used, encoded.len());
    }
}
