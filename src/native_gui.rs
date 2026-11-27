//! Safe Rust wrappers around the native Win32 GUI core.

use crate::stdlib::parse_json;
use crate::value::Value;
use std::ffi::{CString, c_char, c_int};

unsafe extern "C" {
    fn azra_gui_window(title: *const c_char, width: c_int, height: c_int) -> i64;
    fn azra_gui_label(
        window: i64,
        text: *const c_char,
        x: c_int,
        y: c_int,
        w: c_int,
        h: c_int,
    ) -> c_int;
    fn azra_gui_button(
        window: i64,
        text: *const c_char,
        x: c_int,
        y: c_int,
        w: c_int,
        h: c_int,
    ) -> c_int;
    fn azra_gui_edit(
        window: i64,
        text: *const c_char,
        x: c_int,
        y: c_int,
        w: c_int,
        h: c_int,
    ) -> c_int;
    fn azra_gui_set_text(window: i64, id: c_int, text: *const c_char) -> c_int;
    fn azra_gui_get_text(window: i64, id: c_int, out: *mut c_char, capacity: c_int) -> c_int;
    fn azra_gui_show(window: i64) -> c_int;
    fn azra_gui_pump(window: i64, out: *mut c_char, capacity: c_int) -> c_int;
    fn azra_gui_message(text: *const c_char, title: *const c_char) -> c_int;
    fn azra_gui_close(window: i64) -> c_int;
}

fn text(value: &str) -> Result<CString, String> {
    CString::new(value).map_err(|e| e.to_string())
}

#[derive(Debug, Clone)]
pub struct GuiEvent {
    pub kind: i64,
    pub id: i64,
    pub text: String,
}

fn event_field(value: &Value, key: &str) -> i64 {
    match value {
        Value::Map(map) => match map.borrow().get(key) {
            Some(Value::Number(n)) => n.as_f64() as i64,
            _ => 0,
        },
        _ => 0,
    }
}

fn event_text(value: &Value) -> String {
    match value {
        Value::Map(map) => match map.borrow().get("text") {
            Some(Value::Str(s)) => s.clone(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

pub struct Window(i64);

impl Window {
    pub fn create(title: &str, width: i32, height: i32) -> Result<Self, String> {
        let title = text(title)?;
        let handle = unsafe { azra_gui_window(title.as_ptr(), width, height) };
        if handle < 0 {
            Err("native GUI is only available on Windows builds with the C++ core".to_string())
        } else {
            Ok(Window(handle))
        }
    }

    pub fn label(&self, value: &str, x: i32, y: i32, w: i32, h: i32) -> Result<i32, String> {
        let value = text(value)?;
        let id = unsafe { azra_gui_label(self.0, value.as_ptr(), x, y, w, h) };
        if id < 0 {
            Err("could not create label".to_string())
        } else {
            Ok(id)
        }
    }

    pub fn button(&self, value: &str, x: i32, y: i32, w: i32, h: i32) -> Result<i32, String> {
        let value = text(value)?;
        let id = unsafe { azra_gui_button(self.0, value.as_ptr(), x, y, w, h) };
        if id < 0 {
            Err("could not create button".to_string())
        } else {
            Ok(id)
        }
    }

    pub fn edit(&self, value: &str, x: i32, y: i32, w: i32, h: i32) -> Result<i32, String> {
        let value = text(value)?;
        let id = unsafe { azra_gui_edit(self.0, value.as_ptr(), x, y, w, h) };
        if id < 0 {
            Err("could not create edit box".to_string())
        } else {
            Ok(id)
        }
    }

    pub fn set_text(&self, id: i32, value: &str) -> Result<(), String> {
        let value = text(value)?;
        if unsafe { azra_gui_set_text(self.0, id, value.as_ptr()) } == 0 {
            Ok(())
        } else {
            Err("unknown control id".to_string())
        }
    }

    pub fn get_text(&self, id: i32) -> Result<String, String> {
        let mut buffer = vec![0u8; 4096];
        let n = unsafe {
            azra_gui_get_text(
                self.0,
                id,
                buffer.as_mut_ptr().cast::<c_char>(),
                buffer.len() as c_int,
            )
        };
        if n < 0 {
            Err("unknown control id".to_string())
        } else {
            Ok(String::from_utf8_lossy(&buffer[..n as usize]).to_string())
        }
    }

    pub fn show(&self) -> Result<(), String> {
        if unsafe { azra_gui_show(self.0) } == 0 {
            Ok(())
        } else {
            Err("could not show window".to_string())
        }
    }

    /// Non-blocking event poll. `None` means no pending event; `Some(closed)`
    /// is signalled as `kind == 3` by the event loop.
    pub fn pump(&self) -> Result<Option<GuiEvent>, String> {
        let mut buffer = vec![0u8; 8192];
        let code = unsafe {
            azra_gui_pump(
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
                let value = parse_json(&json, 0).map_err(|e| e.to_string())?;
                Ok(Some(GuiEvent {
                    kind: event_field(&value, "type"),
                    id: event_field(&value, "id"),
                    text: event_text(&value),
                }))
            }
            _ => Err("native GUI pump failed".to_string()),
        }
    }

    pub fn close(self) {
        unsafe {
            azra_gui_close(self.0);
        }
    }
}

pub fn message_box(value: &str, title: &str) -> Result<(), String> {
    let value = text(value)?;
    let title = text(title)?;
    if unsafe { azra_gui_message(value.as_ptr(), title.as_ptr()) } == 0 {
        Ok(())
    } else {
        Err("native message boxes are only available on Windows".to_string())
    }
}
