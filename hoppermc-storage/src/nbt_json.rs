use fastnbt::{ByteArray, IntArray, LongArray, Value};
use serde_json::{Map, Number, Value as JsonValue};
use std::collections::HashMap;

pub fn nbt_to_json(nbt: Value) -> JsonValue {
    match nbt {
        Value::Compound(c) => {
            let mut map = Map::new();
            for (k, v) in c {
                map.insert(k, nbt_to_json(v));
            }
            JsonValue::Object(map)
        }
        Value::List(l) => JsonValue::Array(l.into_iter().map(nbt_to_json).collect()),
        Value::String(s) => JsonValue::String(s),
        Value::Byte(b) => JsonValue::Number(b.into()),
        Value::Short(s) => JsonValue::Number(s.into()),
        Value::Int(i) => JsonValue::Number(i.into()),
        Value::Long(l) => JsonValue::Number(l.into()),
        Value::Float(f) => JsonValue::Number(Number::from_f64(f as f64).unwrap_or(Number::from(0))),
        Value::Double(d) => JsonValue::Number(Number::from_f64(d).unwrap_or(Number::from(0))),
        Value::ByteArray(ba) => {
            let mut map = Map::new();
            map.insert(
                "__fastnbt_byte_array".to_string(),
                JsonValue::Array(ba.iter().map(|&b| JsonValue::Number(b.into())).collect()),
            );
            JsonValue::Object(map)
        }
        Value::IntArray(ia) => {
            let mut map = Map::new();
            map.insert(
                "__fastnbt_int_array".to_string(),
                JsonValue::Array(ia.iter().map(|&i| JsonValue::Number(i.into())).collect()),
            );
            JsonValue::Object(map)
        }
        Value::LongArray(la) => {
            let mut map = Map::new();
            map.insert(
                "__fastnbt_long_array".to_string(),
                JsonValue::Array(la.iter().map(|&l| JsonValue::Number(l.into())).collect()),
            );
            JsonValue::Object(map)
        }
    }
}

pub fn json_to_nbt(json: JsonValue) -> Value {
    match json {
        JsonValue::Object(map) => {
            // Check for special tags
            if map.len() == 1 {
                // Try native fastnbt restoration first (handles byte-oriented arrays from previous versions)
                if map.contains_key("__fastnbt_byte_array") || 
                   map.contains_key("__fastnbt_int_array") || 
                   map.contains_key("__fastnbt_long_array") {
                    
                    if let Ok(v) = serde_json::from_value::<Value>(JsonValue::Object(map.clone())) {
                        return v;
                    }

                    // Fallback to our "human readable" format (array of ints)
                    if let Some(JsonValue::Array(arr)) = map.get("__fastnbt_byte_array") {
                        let vec: Vec<i8> = arr.iter().filter_map(|v| v.as_i64().map(|i| i as i8)).collect();
                        return Value::ByteArray(ByteArray::new(vec));
                    }
                    if let Some(JsonValue::Array(arr)) = map.get("__fastnbt_int_array") {
                        let vec: Vec<i32> = arr.iter().filter_map(|v| v.as_i64().map(|i| i as i32)).collect();
                        return Value::IntArray(IntArray::new(vec));
                    }
                    if let Some(JsonValue::Array(arr)) = map.get("__fastnbt_long_array") {
                        let vec: Vec<i64> = arr.iter().filter_map(|v| v.as_i64()).collect();
                        return Value::LongArray(LongArray::new(vec));
                    }
                }
            }

            let mut compound = HashMap::new();
            for (k, v) in map {
                compound.insert(k, json_to_nbt(v));
            }
            Value::Compound(compound)
        }
        JsonValue::Array(arr) => Value::List(arr.into_iter().map(json_to_nbt).collect()),
        JsonValue::String(s) => Value::String(s),
        JsonValue::Number(num) => {
            if let Some(i) = num.as_i64() {
                Value::Long(i)
            } else if let Some(f) = num.as_f64() {
                Value::Double(f)
            } else {
                Value::Double(0.0)
            }
        }
        JsonValue::Bool(b) => Value::Byte(if b { 1 } else { 0 }),
        JsonValue::Null => Value::Byte(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastnbt::{LongArray, Value};

    #[test]
    fn test_long_array_roundtrip() {
        let longs = vec![1, 2, 3, 4096];
        let nbt = Value::LongArray(LongArray::new(longs.clone()));
        let json = nbt_to_json(nbt.clone());
        let restored = json_to_nbt(json);
        
        if let Value::LongArray(la) = restored {
            assert_eq!(la.iter().copied().collect::<Vec<_>>(), longs);
        } else {
            panic!("Restored as wrong type: {:?}", restored);
        }
    }

    #[test]
    fn test_native_bytes_restoration() {
        // Simulates data saved by fastnbt's native Serialize (8 bytes per long)
        let json = serde_json::json!({
            "__fastnbt_long_array": [0,0,0,0,0,0,0,1, 0,0,0,0,0,0,0,2]
        });
        let restored = json_to_nbt(json);
        if let Value::LongArray(la) = restored {
            assert_eq!(la.iter().copied().collect::<Vec<_>>(), vec![1, 2]);
        } else {
            panic!("Failed to restore native bytes: {:?}", restored);
        }
    }

    #[test]
    fn test_legacy_list_restoration() {
        // Simulates even older data or simple lists
        let json = serde_json::json!([1, 2, 3]);
        let restored = json_to_nbt(json);
        if let Value::List(l) = restored {
            assert_eq!(l.len(), 3);
        } else {
            panic!("Restored as wrong type: {:?}", restored);
        }
    }
}
