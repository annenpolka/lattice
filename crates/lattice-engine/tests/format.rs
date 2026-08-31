use lattice_engine::Engine;
use serde_json::Value;

#[test]
fn formatting_preserves_core_ir_except_source_coordinates() {
    let source = r#"project   "demo"
media game "capture.mp4"
sequence main{demo}
// generic invocation names remain opaque
scene demo{game [ 10s .. 20s ] as fight
freeze fight at 5.2s for 1.5s
title "Hello"{at 2s for 3s
opacity 90}}
"#;
    let engine = Engine::default();
    let before = engine.compile(source).unwrap();
    let formatted = engine.format_vel(source).unwrap();
    let after = engine.compile(&formatted).unwrap();

    let mut before_ir = serde_json::to_value(before.project).unwrap();
    let mut after_ir = serde_json::to_value(after.project).unwrap();
    clear_source_coordinates(&mut before_ir);
    clear_source_coordinates(&mut after_ir);
    assert_eq!(after_ir, before_ir);
    assert_eq!(engine.format_vel(&formatted).unwrap(), formatted);
    assert!(formatted.contains("// generic invocation names remain opaque"));
}

fn clear_source_coordinates(value: &mut Value) {
    match value {
        Value::Array(values) => {
            for value in values {
                clear_source_coordinates(value);
            }
        }
        Value::Object(object) => {
            if let Some(Value::Object(provenance)) = object.get_mut("provenance") {
                provenance.insert("span".into(), Value::Null);
            }
            for value in object.values_mut() {
                clear_source_coordinates(value);
            }
        }
        _ => {}
    }
}
