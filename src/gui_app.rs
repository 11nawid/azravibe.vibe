//! Native GUI application host: Win32 window + Azra `when gui event:` handlers.

use crate::error::AzraError;
use crate::native_gui::Window;
use crate::value::Value;
use crate::{AzraEngine, Num, SandboxConfig};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::thread::sleep;
use std::time::Duration;

fn text_arg(args: &[Value], index: usize, what: &str) -> Result<String, AzraError> {
    args.get(index)
        .ok_or_else(|| AzraError::runtime(format!("missing {what}")))?
        .as_str(what, 0)
        .map(|value| value.to_string())
}

fn int_arg(args: &[Value], index: usize, what: &str) -> Result<i32, AzraError> {
    Ok(args
        .get(index)
        .ok_or_else(|| AzraError::runtime(format!("missing {what}")))?
        .as_number(what, 0)?
        .as_f64() as i32)
}

pub struct NativeApp {
    engine: AzraEngine,
    window: Rc<RefCell<Option<Window>>>,
}

impl NativeApp {
    pub fn new(path: &Path, title: &str, width: i32, height: i32) -> Result<Self, String> {
        let mut engine = AzraEngine::with_sandbox(SandboxConfig {
            max_call_depth: 512,
            ..SandboxConfig::permissive()
        });
        crate::host::register_common(&mut engine);
        let window = Rc::new(RefCell::new(Some(Window::create(title, width, height)?)));

        let label_window = window.clone();
        engine.register_function("gui_label", move |args| {
            let value = text_arg(args, 0, "label text")?;
            let x = int_arg(args, 1, "x")?;
            let y = int_arg(args, 2, "y")?;
            let w = int_arg(args, 3, "width")?;
            let h = int_arg(args, 4, "height")?;
            let borrowed = label_window.borrow();
            let active = borrowed
                .as_ref()
                .ok_or_else(|| AzraError::runtime("window is closed"))?;
            let id = active
                .label(&value, x, y, w, h)
                .map_err(AzraError::runtime)?;
            Ok(Value::Number(Num::Int(id as i64)))
        });

        let button_window = window.clone();
        engine.register_function("gui_button", move |args| {
            let value = text_arg(args, 0, "button label")?;
            let x = int_arg(args, 1, "x")?;
            let y = int_arg(args, 2, "y")?;
            let w = int_arg(args, 3, "width")?;
            let h = int_arg(args, 4, "height")?;
            let borrowed = button_window.borrow();
            let active = borrowed
                .as_ref()
                .ok_or_else(|| AzraError::runtime("window is closed"))?;
            let id = active
                .button(&value, x, y, w, h)
                .map_err(AzraError::runtime)?;
            Ok(Value::Number(Num::Int(id as i64)))
        });

        let edit_window = window.clone();
        engine.register_function("gui_edit", move |args| {
            let value = text_arg(args, 0, "initial text")?;
            let x = int_arg(args, 1, "x")?;
            let y = int_arg(args, 2, "y")?;
            let w = int_arg(args, 3, "width")?;
            let h = int_arg(args, 4, "height")?;
            let borrowed = edit_window.borrow();
            let active = borrowed
                .as_ref()
                .ok_or_else(|| AzraError::runtime("window is closed"))?;
            let id = active
                .edit(&value, x, y, w, h)
                .map_err(AzraError::runtime)?;
            Ok(Value::Number(Num::Int(id as i64)))
        });

        let set_window = window.clone();
        engine.register_function("gui_set_text", move |args| {
            let id = int_arg(args, 0, "control id")?;
            let value = text_arg(args, 1, "text")?;
            let borrowed = set_window.borrow();
            let active = borrowed
                .as_ref()
                .ok_or_else(|| AzraError::runtime("window is closed"))?;
            active.set_text(id, &value).map_err(AzraError::runtime)?;
            Ok(Value::Null)
        });

        let get_window = window.clone();
        engine.register_function("gui_get_text", move |args| {
            let id = int_arg(args, 0, "control id")?;
            let borrowed = get_window.borrow();
            let active = borrowed
                .as_ref()
                .ok_or_else(|| AzraError::runtime("window is closed"))?;
            Ok(Value::Str(active.get_text(id).map_err(AzraError::runtime)?))
        });

        engine.register_function("gui_message", |args| {
            let value = text_arg(args, 0, "message")?;
            let title = args
                .get(1)
                .map(|v| v.render())
                .unwrap_or_else(|| "Azra".to_string());
            crate::native_gui::message_box(&value, &title).map_err(AzraError::runtime)?;
            Ok(Value::Null)
        });

        engine.eval_file(path).map_err(|e| e.to_string())?;
        {
            let borrowed = window.borrow();
            let active = borrowed.as_ref().expect("window exists");
            active.show()?;
        }
        Ok(NativeApp { engine, window })
    }

    pub fn run(&mut self) -> Result<(), String> {
        loop {
            let event = {
                let borrowed = self.window.borrow();
                let active = borrowed
                    .as_ref()
                    .ok_or_else(|| "window is closed".to_string())?;
                active.pump().map_err(|e| e.to_string())?
            };
            match event {
                Some(event) => {
                    let value = Value::map(vec![
                        ("type".to_string(), Value::Number(Num::Int(event.kind))),
                        ("id".to_string(), Value::Number(Num::Int(event.id))),
                        ("text".to_string(), Value::Str(event.text)),
                    ]);
                    let result = self.engine.trigger_event_with_args("gui event", &[value]);
                    for line in self.engine.take_output() {
                        println!("  | {}", line.text);
                    }
                    if let Err(err) = result {
                        eprintln!("GUI handler error: {err}");
                    }
                    if event.kind == 3 {
                        break;
                    }
                }
                None => sleep(Duration::from_millis(10)),
            }
        }
        Ok(())
    }
}

impl Drop for NativeApp {
    fn drop(&mut self) {
        if let Some(window) = self.window.borrow_mut().take() {
            window.close();
        }
    }
}
