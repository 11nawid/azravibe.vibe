//! Azra HTTP server framework: C++/Rust networking core + Azra request handlers.
//!
//! A `.vibe` app registers `when http request with request:` handlers and uses
//! the `respond`, `http_fetch` and `http_get` host functions. The CLI command
//! `azra serve app.vibe --port 8080` drives the request loop.

use crate::error::AzraError;
use crate::native::{self, HttpRequest};
use crate::value::Value;
use crate::{AzraEngine, SandboxConfig};
use std::cell::RefCell;
use std::rc::Rc;
use std::thread::sleep;
use std::time::Duration;

fn map_field(value: &Value, key: &str) -> Option<Value> {
    match value {
        Value::Map(map) => map.borrow().get(key).cloned(),
        _ => None,
    }
}

fn str_arg(args: &[Value], index: usize, what: &str) -> Result<String, AzraError> {
    args.get(index)
        .ok_or_else(|| AzraError::runtime(format!("missing {what}")))?
        .as_str(what, 0)
        .map(|value| value.to_string())
}

fn int_arg(args: &[Value], index: usize, what: &str) -> Result<i64, AzraError> {
    Ok(args
        .get(index)
        .ok_or_else(|| AzraError::runtime(format!("missing {what}")))?
        .as_number(what, 0)?
        .as_f64() as i64)
}

fn request_value(request: &HttpRequest) -> Value {
    Value::map(vec![
        ("id".to_string(), Value::Number(crate::Num::Int(request.id))),
        ("method".to_string(), Value::Str(request.method.clone())),
        ("path".to_string(), Value::Str(request.path.clone())),
        ("query".to_string(), Value::Str(request.query.clone())),
        ("headers".to_string(), Value::Str(request.headers.clone())),
        ("body".to_string(), Value::Str(request.body.clone())),
    ])
}

/// A loaded Azra app plus its HTTP server.
pub struct ServerApp {
    engine: AzraEngine,
    server: Rc<RefCell<Option<native::Server>>>,
    responded: Rc<RefCell<bool>>,
    port: u16,
}

impl ServerApp {
    /// Load `path` and start listening on `port`.
    pub fn new(path: &std::path::Path, port: u16) -> Result<Self, String> {
        let mut engine = AzraEngine::with_sandbox(SandboxConfig {
            max_call_depth: 512,
            ..SandboxConfig::permissive()
        });
        let server = Rc::new(RefCell::new(Some(
            native::Server::listen(port).map_err(|e| format!("cannot start HTTP server: {e}"))?,
        )));
        let responded = Rc::new(RefCell::new(false));

        let respond_server = server.clone();
        let respond_flag = responded.clone();
        engine.register_function("respond", move |args| {
            let request = args
                .first()
                .ok_or_else(|| AzraError::runtime("`respond` needs a request"))?;
            let id = match map_field(request, "id") {
                Some(Value::Number(n)) => n.as_f64() as i64,
                _ => return Err(AzraError::runtime("`respond` needs a request map")),
            };
            let status = int_arg(args, 1, "status code")? as i32;
            let body = str_arg(args, 2, "response body")?;
            let content_type = args
                .get(3)
                .map(|v| v.render())
                .unwrap_or_else(|| "text/plain; charset=utf-8".to_string());
            let borrowed = respond_server.borrow();
            let active = borrowed
                .as_ref()
                .ok_or_else(|| AzraError::runtime("HTTP server is closed"))?;
            active
                .respond(id, status, &content_type, &body)
                .map_err(AzraError::runtime)?;
            *respond_flag.borrow_mut() = true;
            Ok(Value::Null)
        });

        engine.register_function("http_fetch", move |args| {
            let method = str_arg(args, 0, "HTTP method")?;
            let url = str_arg(args, 1, "URL")?;
            let headers = args.get(2).map(|v| v.render()).unwrap_or_default();
            let body = args.get(3).map(|v| v.render()).unwrap_or_default();
            let response =
                native::http_request(&method, &url, &headers, &body).map_err(AzraError::runtime)?;
            Ok(Value::map(vec![
                (
                    "status".to_string(),
                    Value::Number(crate::Num::Int(response.status)),
                ),
                ("headers".to_string(), Value::Str(response.headers)),
                ("body".to_string(), Value::Str(response.body)),
            ]))
        });

        engine.register_function("http_get", |args| {
            let url = str_arg(args, 0, "URL")?;
            let response = native::http_request("GET", &url, "", "").map_err(AzraError::runtime)?;
            Ok(Value::Str(response.body))
        });

        engine.eval_file(path).map_err(|e| e.to_string())?;
        Ok(ServerApp {
            engine,
            server,
            responded,
            port,
        })
    }

    /// Serve requests until the process is stopped.
    pub fn run(&mut self) -> Result<(), String> {
        println!("Azra server listening on http://0.0.0.0:{}", self.port);
        loop {
            let request = {
                let borrowed = self.server.borrow();
                let active = borrowed
                    .as_ref()
                    .ok_or_else(|| "HTTP server is closed".to_string())?;
                active.poll().map_err(|e| e.to_string())?
            };
            let Some(request) = request else {
                sleep(Duration::from_millis(10));
                continue;
            };
            *self.responded.borrow_mut() = false;
            let value = request_value(&request);
            let handler_result = self
                .engine
                .trigger_event_with_args("http request", &[value.clone()]);
            for line in self.engine.take_output() {
                println!("  | {}", line.text);
            }
            if let Err(err) = handler_result {
                eprintln!("handler error: {err}");
            }
            if !*self.responded.borrow() {
                let borrowed = self.server.borrow();
                if let Some(active) = borrowed.as_ref() {
                    let _ = active.respond(
                        request.id,
                        404,
                        "text/plain; charset=utf-8",
                        "404 Not Found — no `when http request:` handler responded",
                    );
                }
            }
        }
    }
}

impl Drop for ServerApp {
    fn drop(&mut self) {
        if let Some(server) = self.server.borrow_mut().take() {
            server.close();
        }
    }
}
