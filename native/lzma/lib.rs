//! One-shot decoding through the bundled, statically linked liblzma.
use lzma_sys::*;

struct Stream(lzma_stream);

impl Drop for Stream {
    // Release the decoder state on success and every error path.
    fn drop(&mut self) { unsafe { lzma_end(&mut self.0); } }
}

// Decode one stream and retain its consumed length for concatenated input.
pub fn decode(input: &[u8], format: i64, limit: u64) -> Result<(Vec<u8>, usize), i32> {
    let mut state = Stream(unsafe { std::mem::zeroed() });
    let status = unsafe {
        match format {
            0 => lzma_auto_decoder(&mut state.0, limit, 0),
            1 => lzma_stream_decoder(&mut state.0, limit, 0),
            2 => lzma_alone_decoder(&mut state.0, limit),
            _ => return Err(8),
        }
    };
    if status != 0 { return Err(status as i32); }
    state.0.next_in = input.as_ptr();
    state.0.avail_in = input.len();
    let mut output = Vec::new();
    loop {
        let mut chunk = [0u8; 16384];
        state.0.next_out = chunk.as_mut_ptr();
        state.0.avail_out = chunk.len();
        let status = unsafe { lzma_code(&mut state.0, 3) };
        let count = chunk.len() - state.0.avail_out;
        output.try_reserve(count).map_err(|_| 5)?;
        output.extend_from_slice(&chunk[..count]);
        match status {
            0 => (),
            1 => return Ok((output, input.len() - state.0.avail_in)),
            fault => return Err(fault as i32),
        }
    }
}

// Encode an XZ stream with the bundled codec's validated preset and check.
pub fn encode(input: &[u8], preset: u32, check: i32) -> Result<Vec<u8>, i32> {
    let capacity = unsafe { lzma_stream_buffer_bound(input.len()) };
    let mut output = Vec::new();
    output.try_reserve_exact(capacity).map_err(|_| 5)?;
    output.resize(capacity, 0);
    let mut used = 0;
    let status = unsafe { lzma_easy_buffer_encode(preset, check as lzma_check, std::ptr::null(),
        input.as_ptr(), input.len(), output.as_mut_ptr(), &mut used, capacity) };
    if status != 0 { return Err(status as i32); }
    output.truncate(used);
    Ok(output)
}

#[cfg(test)]
mod tests {
    // Round-trip real compressed data through the bundled codec.
    #[test]
    fn xz_roundtrip() {
        let raw = b"lumen compressed archive";
        let encoded = super::encode(raw, 6, 4).unwrap();
        let (decoded, used) = super::decode(&encoded, 1, u64::MAX).unwrap();
        assert_eq!(decoded, raw);
        assert_eq!(used, encoded.len());
    }
}
