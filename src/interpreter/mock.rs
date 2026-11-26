//! Mock engine objects: default instances of registered engine types so
//! game scripts (`make rounded_box as block`) run standalone.

use crate::token::Num;
use crate::value::{ObjectData, Value};

/// Create a default instance of a registered engine type.
pub(super) fn mock_object(type_name: &str) -> ObjectData {
    let mut obj = ObjectData::new(type_name);
    let int = |i: i64| Value::Number(Num::Int(i));
    match type_name {
        "rounded_box" => {
            obj.set("color", Value::Str("white".to_string()));
            obj.set("material", Value::Str("default".to_string()));
            obj.set("position", Value::list(vec![int(0), int(0), int(0)]));
            obj.set("size", Value::list(vec![int(1), int(1), int(1)]));
            obj.set("anchored", Value::Bool(true));
            obj.set("visible", Value::Bool(true));
        }
        "sound" => {
            obj.set("file", Value::Str(String::new()));
            obj.set("volume", Value::Number(Num::Float(1.0)));
        }
        "player" => {
            obj.set("name", Value::Str("player".to_string()));
            obj.set("health", int(100));
            obj.set("position", Value::list(vec![int(0), int(0), int(0)]));
        }
        _ => {}
    }
    obj
}
