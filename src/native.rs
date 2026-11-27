//! Networking bridge: the C++ core in `native/` when available, otherwise a
//! pure-Rust fallback with the same API. Both produce identical Azra values.

use crate::stdlib::parse_json;
use crate::value::Value;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub id: i64,
    pub method: String,
    pub path: String,
    pub query: String,
    pub headers: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: i64,
    pub headers: String,
    pub body: String,
}

fn field(value: &Value, key: &str) -> Option<Value> {
    match value {
        Value::Map(map) => map.borrow().get(key).cloned(),
        _ => None,
    }
}

fn field_int(value: &Value, key: &str) -> i64 {
    match field(value, key) {
        Some(Value::Number(n)) => n.as_f64() as i64,
        _ => 0,
    }
}

fn field_str(value: &Value, key: &str) -> String {
    match field(value, key) {
        Some(Value::Str(s)) => s,
        _ => String::new(),
    }
}

fn parse_request(json: &str) -> Result<HttpRequest, String> {
    let value = parse_json(json, 0).map_err(|e| e.to_string())?;
    Ok(HttpRequest {
        id: field_int(&value, "id"),
        method: field_str(&value, "method"),
        path: field_str(&value, "path"),
        query: field_str(&value, "query"),
        headers: field_str(&value, "headers"),
        body: field_str(&value, "body"),
    })
}

fn parse_response(json: &str) -> Result<HttpResponse, String> {
    let value = parse_json(json, 0).map_err(|e| e.to_string())?;
    Ok(HttpResponse {
        status: field_int(&value, "status"),
        headers: field_str(&value, "headers"),
        body: field_str(&value, "body"),
    })
}

#[cfg(azra_native_cpp)]
mod backend {
    use super::*;
    use std::ffi::{CString, c_char, c_int};

    unsafe extern "C" {
        fn azra_net_listen(port: c_int) -> i64;
        fn azra_net_poll(server: i64, out: *mut c_char, capacity: c_int) -> c_int;
        fn azra_net_respond(
            server: i64,
            request_id: i64,
            status: c_int,
            content_type: *const c_char,
            body: *const c_char,
            body_len: c_int,
        ) -> c_int;
        fn azra_net_request(
            method: *const c_char,
            url: *const c_char,
            headers: *const c_char,
            body: *const c_char,
            out: *mut c_char,
            capacity: c_int,
        ) -> c_int;
        fn azra_net_close(server: i64) -> c_int;
    }

    fn text(value: &str) -> Result<CString, String> {
        CString::new(value).map_err(|e| e.to_string())
    }

    pub struct Server(i64);

    impl Server {
        pub fn listen(port: u16) -> Result<Self, String> {
            let handle = unsafe { azra_net_listen(port as c_int) };
            if handle < 0 {
                Err(format!("cannot listen on port {port}"))
            } else {
                Ok(Server(handle))
            }
        }

        pub fn poll(&self) -> Result<Option<HttpRequest>, String> {
            let mut buffer = vec![0u8; 1 << 20];
            let code = unsafe {
                azra_net_poll(
                    self.0,
                    buffer.as_mut_ptr().cast::<c_char>(),
                    buffer.len() as c_int,
                )
            };
            match code {
                0 => Ok(None),
                1 => {
                    let end = buffer.iter().position(|b| *b == 0).unwrap_or(buffer.len());
                    let json = String::from_utf8_lossy(&buffer[..end]);
                    Ok(Some(parse_request(&json)?))
                }
                _ => Err("failed to poll the HTTP server".to_string()),
            }
        }

        pub fn respond(
            &self,
            id: i64,
            status: i32,
            content_type: &str,
            body: &str,
        ) -> Result<(), String> {
            let content_type = text(content_type)?;
            let body = text(body)?;
            let code = unsafe {
                azra_net_respond(
                    self.0,
                    id,
                    status,
                    content_type.as_ptr(),
                    body.as_ptr(),
                    body.as_bytes().len() as c_int,
                )
            };
            if code == 0 {
                Ok(())
            } else {
                Err("no pending request with that id".to_string())
            }
        }

        pub fn close(&self) {
            unsafe {
                azra_net_close(self.0);
            }
        }
    }

    pub fn http_request(
        method: &str,
        url: &str,
        headers: &str,
        body: &str,
    ) -> Result<HttpResponse, String> {
        let (method, url, headers, body) = (text(method)?, text(url)?, text(headers)?, text(body)?);
        let mut buffer = vec![0u8; 4 << 20];
        let status = unsafe {
            azra_net_request(
                method.as_ptr(),
                url.as_ptr(),
                headers.as_ptr(),
                body.as_ptr(),
                buffer.as_mut_ptr().cast::<c_char>(),
                buffer.len() as c_int,
            )
        };
        if status < 0 {
            return Err(format!("HTTP request to {url:?} failed"));
        }
        let end = buffer.iter().position(|b| *b == 0).unwrap_or(buffer.len());
        let response = parse_response(&String::from_utf8_lossy(&buffer[..end]))?;
        Ok(response)
    }
}

#[cfg(not(azra_native_cpp))]
mod backend {
    use super::*;
    use std::collections::BTreeMap;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::Mutex;
    use std::time::Duration;

    pub struct Server {
        listener: TcpListener,
        pending: Mutex<BTreeMap<i64, (TcpStream, HttpRequest)>>,
        next_id: Mutex<i64>,
    }

    impl Server {
        pub fn listen(port: u16) -> Result<Self, String> {
            let listener = TcpListener::bind(("0.0.0.0", port))
                .map_err(|e| format!("cannot listen on port {port}: {e}"))?;
            listener.set_nonblocking(true).map_err(|e| e.to_string())?;
            Ok(Server {
                listener,
                pending: Mutex::new(BTreeMap::new()),
                next_id: Mutex::new(1),
            })
        }

        pub fn poll(&self) -> Result<Option<HttpRequest>, String> {
            let mut socket = match self.listener.accept() {
                Ok((stream, _)) => stream,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(None),
                Err(e) => return Err(e.to_string()),
            };
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .map_err(|e| e.to_string())?;
            let mut raw = Vec::new();
            let mut buffer = [0u8; 4096];
            let split = loop {
                let n = socket.read(&mut buffer).map_err(|e| e.to_string())?;
                if n == 0 {
                    return Ok(None);
                }
                raw.extend_from_slice(&buffer[..n]);
                if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    break pos + 4;
                }
                if raw.len() > 16 << 20 {
                    return Ok(None);
                }
            };
            let head = String::from_utf8_lossy(&raw[..split]);
            let mut lines = head.split("\r\n");
            let request_line = lines.next().unwrap_or("");
            let mut parts = request_line.split_whitespace();
            let method = parts.next().unwrap_or("GET").to_string();
            let target = parts.next().unwrap_or("/").to_string();
            let headers = lines.collect::<Vec<_>>().join("\n");
            let length = headers
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    if key.eq_ignore_ascii_case("content-length") {
                        value.trim().parse::<usize>().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            while raw.len() - split < length {
                let n = socket.read(&mut buffer).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                raw.extend_from_slice(&buffer[..n]);
            }
            let body = String::from_utf8_lossy(&raw[split..split + length]).to_string();
            let (path, query) = match target.split_once('?') {
                Some((p, q)) => (p.to_string(), q.to_string()),
                None => (target, String::new()),
            };
            let mut next = self.next_id.lock().unwrap();
            let id = *next;
            *next += 1;
            drop(next);
            let request = HttpRequest {
                id,
                method,
                path,
                query,
                headers,
                body,
            };
            self.pending
                .lock()
                .unwrap()
                .insert(id, (socket, request.clone()));
            Ok(Some(request))
        }

        pub fn respond(
            &self,
            id: i64,
            status: i32,
            content_type: &str,
            body: &str,
        ) -> Result<(), String> {
            let (mut socket, _) = self
                .pending
                .lock()
                .unwrap()
                .remove(&id)
                .ok_or_else(|| "no pending request with that id".to_string())?;
            let reason = match status {
                200 => "OK",
                201 => "Created",
                400 => "Bad Request",
                404 => "Not Found",
                500 => "Internal Server Error",
                _ => "Status",
            };
            write!(
                socket,
                "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.as_bytes().len()
            )
            .map_err(|e| e.to_string())?;
            socket.flush().map_err(|e| e.to_string())
        }

        pub fn close(&self) {}
    }

    pub fn http_request(
        method: &str,
        url: &str,
        headers: &str,
        body: &str,
    ) -> Result<HttpResponse, String> {
        let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
        let (authority, path) = match rest.split_once('/') {
            Some((a, p)) => (a, format!("/{p}")),
            None => (rest, "/".to_string()),
        };
        let (host, port) = match authority.split_once(':') {
            Some((h, p)) => (h, p.parse::<u16>().unwrap_or(80)),
            None => (authority, 80),
        };
        let mut socket = TcpStream::connect((host, port)).map_err(|e| e.to_string())?;
        write!(
            socket,
            "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n{headers}\r\n{body}"
        )
        .map_err(|e| e.to_string())?;
        socket.flush().map_err(|e| e.to_string())?;
        let mut raw = Vec::new();
        socket.read_to_end(&mut raw).map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&raw);
        let status = text
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let split = text.find("\r\n\r\n").unwrap_or(text.len());
        Ok(HttpResponse {
            status,
            headers: text.lines().skip(1).take_while(|l| !l.is_empty()).collect(),
            body: if split < text.len() {
                text[split + 4..].to_string()
            } else {
                String::new()
            },
        })
    }
}

pub use backend::{Server, http_request};
