//! Stateful zlib codecs, following CPython v3.14.8 Modules/zlibmodule.c.
use libz_sys as z;
use std::{cell::RefCell, collections::HashMap, ffi::CStr};

unsafe extern "C" fn allocate(_: z::voidpf, count: z::uInt, size: z::uInt) -> z::voidpf {
    libc::calloc(count as usize, size as usize)
}
unsafe extern "C" fn release(_: z::voidpf, address: z::voidpf) { libc::free(address); }
fn blank_stream() -> Box<z::z_stream> {
    Box::new(z::z_stream {
        next_in: std::ptr::null_mut(), avail_in: 0, total_in: 0,
        next_out: std::ptr::null_mut(), avail_out: 0, total_out: 0,
        msg: std::ptr::null_mut(), state: std::ptr::null_mut(),
        zalloc: allocate, zfree: release, opaque: std::ptr::null_mut(),
        data_type: 0, adler: 0, reserved: 0,
    })
}

struct Stream {
    raw: Box<z::z_stream>,
    compressing: bool,
    initialized: bool,
}
impl Drop for Stream {
    fn drop(&mut self) {
        if self.initialized { unsafe {
            if self.compressing { z::deflateEnd(&mut *self.raw); }
            else { z::inflateEnd(&mut *self.raw); }
        } }
    }
}
struct Store { next: i64, streams: HashMap<i64, Stream> }
thread_local! { static STORE: RefCell<Store> = RefCell::new(Store { next: 1, streams: HashMap::new() }); }

pub struct Reply {
    pub status: i64, pub output: Vec<u8>, pub consumed: i64,
    pub ended: bool, pub text: String, pub number: i64,
}
impl Reply {
    fn empty() -> Self { Self { status: 0, output: Vec::new(), consumed: 0, ended: false, text: String::new(), number: 0 } }
}
fn description(code: i32, raw: &z::z_stream) -> String {
    if !raw.msg.is_null() { unsafe { CStr::from_ptr(raw.msg).to_string_lossy().into_owned() } }
    else { match code { -5 => "incomplete or truncated stream", -2 => "inconsistent stream state", -3 => "invalid input data", _ => "" }.into() }
}
fn finish_error(reply: &mut Reply, code: i32, stream: &Stream) {
    reply.status = code as i64;
    reply.text = description(code, &stream.raw);
}

/// Private bridge: create compressors/inflaters, advance, flush, copy, release,
/// or calculate checksums. Only interpreter-independent bytes cross this API.
pub fn call(operation: i64, handle: i64, data: &[u8], options: [i64; 5]) -> Reply {
    let mut result = Reply::empty();
    if operation == 0 {
        result.text = unsafe { CStr::from_ptr(z::zlibVersion()).to_string_lossy().into_owned() };
        return result;
    }
    if operation == 7 || operation == 8 {
        let mut sum = options[0] as u32 as z::uLong;
        if data.is_empty() { unsafe {
            sum = if operation == 7 { z::crc32(sum, data.as_ptr(), 0) } else { z::adler32(sum, data.as_ptr(), 0) };
        } }
        for piece in data.chunks(u32::MAX as usize) { unsafe {
            sum = if operation == 7 { z::crc32(sum, piece.as_ptr(), piece.len() as u32) }
            else { z::adler32(sum, piece.as_ptr(), piece.len() as u32) };
        } }
        result.number = sum as u32 as i64;
        return result;
    }
    STORE.with(|storage| {
        let mut storage = storage.borrow_mut();
        if operation == 1 || operation == 2 {
            let mut stream = Stream { raw: blank_stream(), compressing: operation == 1, initialized: false };
            let code = unsafe {
                if stream.compressing {
                    z::deflateInit2_(&mut *stream.raw, options[0] as i32, options[1] as i32, options[2] as i32, options[3] as i32, options[4] as i32, z::zlibVersion(), std::mem::size_of::<z::z_stream>() as i32)
                } else {
                    z::inflateInit2_(&mut *stream.raw, options[0] as i32, z::zlibVersion(), std::mem::size_of::<z::z_stream>() as i32)
                }
            };
            if code != z::Z_OK { finish_error(&mut result, code, &stream); return result; }
            stream.initialized = true;
            if handle != 0 && (stream.compressing || options[0] < 0) {
                let code = unsafe { if stream.compressing { z::deflateSetDictionary(&mut *stream.raw, data.as_ptr(), data.len() as u32) }
                    else { z::inflateSetDictionary(&mut *stream.raw, data.as_ptr(), data.len() as u32) } };
                if code != z::Z_OK { finish_error(&mut result, code, &stream); result.number = -1; return result; }
            }
            let id = storage.next; storage.next += 1;
            storage.streams.insert(id, stream); result.number = id;
            return result;
        }
        if operation == 6 { storage.streams.remove(&handle); return result; }
        if operation == 5 {
            let Some(original) = storage.streams.get_mut(&handle) else { result.status = -2; return result; };
            let mut duplicate = Stream { raw: blank_stream(), compressing: original.compressing, initialized: false };
            let status = unsafe { if original.compressing { z::deflateCopy(&mut *duplicate.raw, &mut *original.raw) } else { z::inflateCopy(&mut *duplicate.raw, &mut *original.raw) } };
            if status != z::Z_OK { finish_error(&mut result, status, original); return result; }
            duplicate.initialized = true;
            let id = storage.next; storage.next += 1;
            storage.streams.insert(id, duplicate); result.number = id;
            return result;
        }
        let Some(stream) = storage.streams.get_mut(&handle) else { result.status = -2; result.text = "inconsistent stream state".into(); return result; };
        if operation == 9 {
            let code = unsafe { z::inflateSetDictionary(&mut *stream.raw, data.as_ptr(), data.len() as u32) };
            if code != z::Z_OK { finish_error(&mut result, code, stream); }
            return result;
        }
        let flush = options[0] as i32;
        let limit = if options[1] <= 0 { usize::MAX } else { options[1] as usize };
        let mut position = 0;
        loop {
            let before = position;
            let chunk = (data.len() - position).min(u32::MAX as usize);
            let capacity = (limit - result.output.len()).min(16 * 1024);
            if capacity == 0 { break; }
            let mut buffer = vec![0u8; capacity];
            stream.raw.next_in = data[position..].as_ptr() as *mut u8;
            stream.raw.avail_in = chunk as u32;
            stream.raw.next_out = buffer.as_mut_ptr(); stream.raw.avail_out = capacity as u32;
            let code = unsafe { if stream.compressing { z::deflate(&mut *stream.raw, flush) } else { z::inflate(&mut *stream.raw, flush) } };
            position += chunk - stream.raw.avail_in as usize;
            let produced = capacity - stream.raw.avail_out as usize;
            result.output.extend_from_slice(&buffer[..produced]);
            if code == z::Z_STREAM_END { result.ended = true; break; }
            if code != z::Z_OK && code != z::Z_BUF_ERROR { finish_error(&mut result, code, stream); break; }
            if stream.raw.avail_out != 0 && position == data.len() { break; }
            if position == before && produced == 0 { break; }
        }
        result.consumed = position as i64;
        // Input and output point into temporary buffers. Clear them so copy()
        // never retains stale addresses between calls.
        stream.raw.next_in = std::ptr::null_mut(); stream.raw.avail_in = 0;
        stream.raw.next_out = std::ptr::null_mut(); stream.raw.avail_out = 0;
        result
    })
}
