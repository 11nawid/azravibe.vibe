//! Minimal WebAssembly bridge for the browser playground.
//!
//! No `wasm-bindgen` is needed: JavaScript writes UTF-8 source into the input
//! buffer, calls `azra_run`, then reads the output buffer. This keeps the
//! crate dependency-free on every target.

use crate::run_source;
use std::cell::UnsafeCell;

const INPUT_CAP: usize = 1024 * 1024;
const OUTPUT_CAP: usize = 4 * 1024 * 1024;

struct Buffer<const N: usize> {
    data: UnsafeCell<[u8; N]>,
}

unsafe impl<const N: usize> Sync for Buffer<N> {}

static INPUT: Buffer<INPUT_CAP> = Buffer {
    data: UnsafeCell::new([0; INPUT_CAP]),
};
static OUTPUT: Buffer<OUTPUT_CAP> = Buffer {
    data: UnsafeCell::new([0; OUTPUT_CAP]),
};
static mut OUTPUT_LEN: usize = 0;

/// Pointer to the input buffer; JavaScript writes source bytes here.
#[unsafe(no_mangle)]
pub extern "C" fn azra_input_ptr() -> *mut u8 {
    INPUT.data.get().cast::<u8>()
}

/// Maximum number of source bytes accepted by [`azra_run`].
#[unsafe(no_mangle)]
pub extern "C" fn azra_input_capacity() -> usize {
    INPUT_CAP
}

/// Pointer to the result buffer (text output or an error message).
#[unsafe(no_mangle)]
pub extern "C" fn azra_output_ptr() -> *mut u8 {
    OUTPUT.data.get().cast::<u8>()
}

/// Number of valid bytes in the result buffer after [`azra_run`].
#[unsafe(no_mangle)]
pub extern "C" fn azra_output_len() -> usize {
    unsafe { OUTPUT_LEN }
}

/// Execute `len` UTF-8 bytes from the input buffer. Returns 0 on success,
/// 1 on an Azra error or 2 on an unexpected panic.
#[unsafe(no_mangle)]
pub extern "C" fn azra_run(len: usize) -> i32 {
    let len = len.min(INPUT_CAP);
    let outcome = std::panic::catch_unwind(|| {
        let input = unsafe { &*INPUT.data.get() }[..len].to_vec();
        match String::from_utf8(input) {
            Err(err) => (1, format!("invalid UTF-8 source: {err}")),
            Ok(source) => match run_source(&source) {
                Ok(text) => (0, text),
                Err(text) => (1, text),
            },
        }
    });
    match outcome {
        Ok((code, text)) => {
            let bytes = text.as_bytes();
            let take = bytes.len().min(OUTPUT_CAP);
            unsafe {
                let out = &mut *OUTPUT.data.get();
                out[..take].copy_from_slice(&bytes[..take]);
                OUTPUT_LEN = take;
            }
            code
        }
        Err(_) => {
            let message = b"internal interpreter panic";
            unsafe {
                let out = &mut *OUTPUT.data.get();
                out[..message.len()].copy_from_slice(message);
                OUTPUT_LEN = message.len();
            }
            2
        }
    }
}
