//! Common host functions shared by the CLI, REPL, server and embedding API.
//! All implementations are dependency-free so Azra works on every target.

use crate::AzraEngine;
use crate::error::AzraError;
use crate::value::Value;
use std::sync::atomic::{AtomicU64, Ordering};

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

fn arg_string(args: &[Value], index: usize, what: &str) -> Result<String, AzraError> {
    args.get(index)
        .ok_or_else(|| AzraError::runtime(format!("missing {what}")))?
        .as_str(what, 0)
        .map(|value| value.to_string())
}

fn arg_or(args: &[Value], index: usize, fallback: &str) -> String {
    args.get(index)
        .map(|v| v.render())
        .unwrap_or_else(|| fallback.to_string())
}

/// Open a URL in the user's default browser.
pub fn open_url(url: &str) -> Result<bool, String> {
    let status = if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .status()
    } else {
        std::process::Command::new("xdg-open").arg(url).status()
    };
    match status {
        Ok(status) if status.success() => Ok(true),
        Ok(status) => Err(format!("could not open browser (exit {status})")),
        Err(e) => Err(format!("could not open browser: {e}")),
    }
}

/// Register the standard host function library on an engine.
pub fn register_common(engine: &mut AzraEngine) {
    engine.register_function("env_get", |args| {
        let name = arg_string(args, 0, "environment variable name")?;
        match std::env::var(&name) {
            Ok(value) => Ok(Value::Str(value)),
            Err(_) => Ok(Value::Null),
        }
    });

    engine.register_function("cli_args", |_args| {
        let args: Vec<Value> = std::env::args().skip(1).map(Value::Str).collect();
        Ok(Value::list(args))
    });

    engine.register_function("arg_at", |args| {
        let index = args
            .get(0)
            .and_then(|v| match v {
                Value::Number(n) => Some(n.as_f64() as usize),
                _ => None,
            })
            .unwrap_or(0);
        let all: Vec<String> = std::env::args().skip(1).collect();
        Ok(all
            .get(index)
            .cloned()
            .map(Value::Str)
            .unwrap_or(Value::Null))
    });

    engine.register_function("now_ms", |_args| {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        Ok(Value::Number(crate::Num::Int(millis)))
    });

    engine.register_function("uuid", |_args| {
        let id = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Ok(Value::Str(format!("{:x}-{:x}", millis, id)))
    });

    engine.register_function("base64_encode", |args| {
        Ok(Value::Str(base64_encode(arg_or(args, 0, "").as_bytes())))
    });

    engine.register_function("base64_decode", |args| {
        let text = arg_or(args, 0, "");
        let bytes = base64_decode(&text).ok_or_else(|| AzraError::runtime("invalid base64"))?;
        Ok(Value::Str(
            String::from_utf8(bytes).map_err(|e| AzraError::runtime(e.to_string()))?,
        ))
    });

    engine.register_function("base64url_encode", |args| {
        Ok(Value::Str(base64url_encode(arg_or(args, 0, "").as_bytes())))
    });

    engine.register_function("base64url_decode", |args| {
        let text = arg_or(args, 0, "").replace('-', "+").replace('_', "/");
        let bytes = base64_decode(&text).ok_or_else(|| AzraError::runtime("invalid base64url"))?;
        Ok(Value::Str(
            String::from_utf8(bytes).map_err(|e| AzraError::runtime(e.to_string()))?,
        ))
    });

    engine.register_function("sha256", |args| {
        Ok(Value::Str(hex(&sha256_bytes(
            arg_or(args, 0, "").as_bytes(),
        ))))
    });

    engine.register_function("hmac_sha256", |args| {
        let key = arg_string(args, 0, "HMAC key")?;
        let message = arg_or(args, 1, "");
        Ok(Value::Str(hex(&hmac_sha256(
            key.as_bytes(),
            message.as_bytes(),
        ))))
    });

    engine.register_function("url_encode", |args| {
        Ok(Value::Str(url_encode(&arg_or(args, 0, ""))))
    });

    engine.register_function("url_decode", |args| {
        Ok(Value::Str(url_decode(&arg_or(args, 0, ""))))
    });

    engine.register_function("https_request", |args| {
        let method = arg_string(args, 0, "HTTP method")?;
        let url = arg_string(args, 1, "URL")?;
        let headers = arg_or(args, 2, "");
        let body = arg_or(args, 3, "");
        let mut command = std::process::Command::new("curl");
        command
            .arg("-sS")
            .arg("-X")
            .arg(&method)
            .arg("-w")
            .arg("\n%{http_code}");
        for header in headers.lines().filter(|line| !line.trim().is_empty()) {
            command.arg("-H").arg(header.trim());
        }
        if !body.is_empty() {
            command.arg("--data-binary").arg(&body);
        }
        command.arg(&url);
        let output = command
            .output()
            .map_err(|e| AzraError::runtime(format!("HTTPS requires curl: {e}")))?;
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        let (payload, status) = match text.rfind('\n') {
            Some(position) => (
                text[..position].to_string(),
                text[position + 1..].trim().parse::<i64>().unwrap_or(0),
            ),
            None => (text.clone(), 0),
        };
        Ok(Value::map(vec![
            ("status".to_string(), Value::Number(crate::Num::Int(status))),
            ("body".to_string(), Value::Str(payload)),
            ("headers".to_string(), Value::Str(String::new())),
        ]))
    });

    engine.register_function("open_browser", |args| {
        let url = arg_string(args, 0, "URL")?;
        open_url(&url).map(Value::Bool).map_err(AzraError::runtime)
    });

    engine.register_function("file_append", |args| {
        let path = arg_string(args, 0, "file path")?;
        let text = arg_or(args, 1, "");
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| AzraError::runtime(format!("cannot open `{path}`: {e}")))?;
        file.write_all(text.as_bytes())
            .map_err(|e| AzraError::runtime(e.to_string()))?;
        Ok(Value::Bool(true))
    });
}

// ------------------------------------------------------------------- base64

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(input: &[u8]) -> String {
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(BASE64[(b0 >> 2) as usize] as char);
        out.push(BASE64[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(BASE64[(((b1 & 15) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(BASE64[(b2 & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut accumulator = 0u32;
    let mut bits = 0u32;
    for byte in input.bytes() {
        if byte == b'=' {
            break;
        }
        let value = BASE64.iter().position(|c| *c == byte)? as u32;
        accumulator = (accumulator << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((accumulator >> bits) as u8);
            accumulator &= (1u32 << bits) - 1;
        }
    }
    Some(out)
}

fn base64url_encode(input: &[u8]) -> String {
    base64_encode(input)
        .replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .to_string()
}

fn url_encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn url_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(value) = u8::from_str_radix(&text[i + 1..i + 3], 16) {
                out.push(value);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(b' ');
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

// -------------------------------------------------------------------- sha256

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn sha256_bytes(input: &[u8]) -> [u8; 32] {
    let mut data = input.to_vec();
    let bit_len = (data.len() as u64) * 8;
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    for block in data.chunks(64) {
        let mut words = [0u32; 64];
        for i in 0..16 {
            words[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = words[i - 15].rotate_right(7)
                ^ words[i - 15].rotate_right(18)
                ^ (words[i - 15] >> 3);
            let s1 = words[i - 2].rotate_right(17)
                ^ words[i - 2].rotate_right(19)
                ^ (words[i - 2] >> 10);
            words[i] = words[i - 16]
                .wrapping_add(s0)
                .wrapping_add(words[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) = (
            state[0], state[1], state[2], state[3], state[4], state[5], state[6], state[7],
        );
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choice = (e & f) ^ ((!e) & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(choice)
                .wrapping_add(K[i])
                .wrapping_add(words[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }
    let mut out = [0u8; 32];
    for (i, word) in state.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut key_block = [0u8; 64];
    if key.len() > 64 {
        key_block[..32].copy_from_slice(&sha256_bytes(key));
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut inner = Vec::with_capacity(64 + message.len());
    inner.extend(key_block.iter().map(|b| b ^ 0x36));
    inner.extend_from_slice(message);
    let inner_hash = sha256_bytes(&inner);
    let mut outer = Vec::with_capacity(96);
    outer.extend(key_block.iter().map(|b| b ^ 0x5c));
    outer.extend_from_slice(&inner_hash);
    sha256_bytes(&outer)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            hex(&sha256_bytes(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(&sha256_bytes(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn base64_round_trip() {
        for text in ["", "a", "azra", "hello world", "日本語"] {
            assert_eq!(
                String::from_utf8(base64_decode(&base64_encode(text.as_bytes())).unwrap()).unwrap(),
                text
            );
        }
    }

    #[test]
    fn url_round_trip() {
        assert_eq!(url_decode(&url_encode("a b&c=د")), "a b&c=د");
    }
}
