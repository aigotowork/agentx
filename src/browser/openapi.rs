//! Shared HTTP projection mechanics only: native integer schemas to existing wire markers.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Empty {}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct HttpError {
    pub error: String,
}

/// Mirror wire::encode without changing native schemas or replacing identity strings.
/// JSON Schema numeric constraints describe native values in metadata; the actual
/// wire field is a strict one-key decimal marker, including nullable integers.
pub(crate) fn marker_document(document: utoipa::openapi::OpenApi) -> Value {
    let mut document = serde_json::to_value(document).expect("OpenAPI is serializable");
    fn project(schema: &mut Value) {
        match schema {
            Value::Object(fields) => {
                let integer = fields.get("type").is_some_and(|ty| ty == "integer" || ty.as_array().is_some_and(|types| types.iter().any(|ty| ty == "integer")));
                if integer {
                    let native = schema.clone();
                    let nullable = native["type"].as_array().is_some_and(|types| types.iter().any(|ty| ty == "null"));
                    let unsigned = native["format"].as_str().is_some_and(|format| format.starts_with("u-")) || native["minimum"].as_u64() == Some(0);
                    let pattern = if unsigned { "^(0|[1-9][0-9]*)$" } else { "^(0|[1-9][0-9]*|-[1-9][0-9]*)$" };
                    let marker = json!({
                        "type": "object", "properties": {"$integer": {"type":"string", "pattern":pattern}},
                        "required":["$integer"], "additionalProperties":false,
                        "x-agentx-native-integer": native["format"].as_str().unwrap_or("int64")
                    });
                    *schema = if nullable { json!({"anyOf":[marker,{"type":"null"}]}) } else { marker };
                    schema["x-agentx-native-schema"] = native.clone();
                    for key in ["description", "title", "deprecated", "readOnly", "writeOnly"] {
                        if let Some(value) = native.get(key) {
                            schema[key] = value.clone();
                        }
                    }
                } else {
                    for value in fields.values_mut() {
                        project(value);
                    }
                }
            }
            Value::Array(items) => items.iter_mut().for_each(project),
            _ => {}
        }
    }
    // Only schema documents are projected; OpenAPI numeric version/status keywords
    // and limits on array lengths are metadata, not serialized resource values.
    if let Some(schemas) = document.pointer_mut("/components/schemas") {
        project(schemas);
    }
    if let Some(paths) = document.get_mut("paths") {
        project(paths);
    }
    document["x-agentx-codec"] = json!({
        "wire":"integer-marker-v1",
        "integerMarker":"$integer",
        "nativeIntegerMetadata":"x-agentx-native-integer",
        "decode":"Every native integer is a single-key marker object containing canonical decimal text; convert schema-declared markers to bigint with native width/sign bounds. Identity strings remain strings.",
        "encode":"Emit native bigint operands as single-key markers; bare numeric tokens, extra keys, -0 and leading zeroes are rejected. Existing strict JSON/native validation remains authoritative."
    });
    document
}

/// Offline export is selected before environment/configuration/native startup.
pub(crate) fn dispatch(args: &[String]) -> Option<i32> {
    let document = match args.first().map(String::as_str)? {
        "__export-management-openapi" => crate::management::openapi::document,
        "__export-workbench-openapi" => crate::collaboration::web::openapi::document,
        "__export-observation-openapi" => crate::observability::web::openapi::document,
        _ => return None,
    };
    if args.len() != 1 {
        eprintln!("OpenAPI export accepts no arguments");
        return Some(2);
    }
    use std::io::Write;
    let result = serde_json::to_writer_pretty(std::io::stdout().lock(), &document()).map_err(std::io::Error::other).and_then(|_| std::io::stdout().write_all(b"\n"));
    Some(if let Err(error) = result {
        eprintln!("OpenAPI export: {error}");
        1
    } else {
        0
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use utoipa::OpenApi;

    #[derive(Serialize, utoipa::ToSchema)]
    struct Integers {
        unsigned: u64,
        signed: i64,
        narrow: u16,
        absent: Option<u64>,
        identity: String,
    }
    #[derive(utoipa::ToSchema)]
    #[allow(dead_code)]
    struct ResultFixture {
        #[schema(value_type = crate::shared::serde_schema::SerdeResult<u64, String>)]
        result: Result<u64, String>,
    }
    #[derive(OpenApi)]
    #[openapi(components(schemas(Integers, ResultFixture)))]
    struct Fixture;

    #[test]
    fn marker_projection_matches_native_wire_and_preserves_integer_width() {
        let document = marker_document(Fixture::openapi());
        let fields = &document["components"]["schemas"]["Integers"]["properties"];
        let input = Integers { unsigned: u64::MAX, signed: i64::MIN, narrow: u16::MAX, absent: None, identity: "18446744073709551615".into() };
        let encoded: Value = serde_json::from_slice(&crate::browser::wire::encode(&input).unwrap()).unwrap();
        assert_eq!(encoded["unsigned"], json!({"$integer":"18446744073709551615"}));
        assert_eq!(encoded["signed"], json!({"$integer":"-9223372036854775808"}));
        assert!(encoded["absent"].is_null());
        assert_eq!(encoded["identity"], "18446744073709551615");
        for (name, format) in [("unsigned", "u-int64"), ("signed", "int64"), ("narrow", "u-int16")] {
            assert_eq!(fields[name]["x-agentx-native-integer"], format);
            assert_eq!(fields[name]["additionalProperties"], false);
            let pattern = regex::Regex::new(fields[name]["properties"]["$integer"]["pattern"].as_str().unwrap()).unwrap();
            assert!(pattern.is_match(encoded[name]["$integer"].as_str().unwrap()));
            for noncanonical in ["-0", "00", "+1", "1.5", "1e3"] {
                assert!(!pattern.is_match(noncanonical));
            }
        }
        assert!(fields["absent"]["anyOf"].as_array().unwrap().iter().any(|schema| schema["type"] == "null"));
        assert_eq!(fields["identity"]["type"], "string");
        let result: Result<u64, String> = Ok(u64::MAX);
        let result_wire: Value = serde_json::from_slice(&crate::browser::wire::encode(&result).unwrap()).unwrap();
        assert_eq!(result_wire, json!({"Ok":{"$integer":"18446744073709551615"}}));
        // ToSchema::name returns the generic base name; the emitted property
        // reference includes its type arguments. Follow the actual derived ref.
        let reference = document["components"]["schemas"]["ResultFixture"]["properties"]["result"]["$ref"].as_str().expect("Result fixture must reference the derived serde Result schema");
        let variants = document.pointer(reference.strip_prefix('#').unwrap()).expect("Result schema reference must resolve")["oneOf"].as_array().expect("serde Result must be an external Ok/Err union");
        assert!(variants.iter().any(|schema| schema["properties"].get("Ok").is_some()));
        assert!(variants.iter().any(|schema| schema["properties"].get("Err").is_some()));
    }

    #[test]
    fn all_listener_specs_cover_closed_public_boundaries() {
        let management = crate::management::openapi::document();
        let workbench = crate::collaboration::web::openapi::document();
        let observation = crate::observability::web::openapi::document();
        let operations = |spec: &Value| spec["paths"].as_object().unwrap().values().map(|path| path.as_object().unwrap().len()).sum::<usize>();
        assert_eq!((operations(&management), operations(&workbench), operations(&observation)), (30, 3, 2));
        assert_eq!(workbench["components"]["schemas"]["Read"]["oneOf"].as_array().unwrap().len(), 5);
        assert_eq!(workbench["components"]["schemas"]["OperatorCommand"]["oneOf"].as_array().unwrap().len(), 9);
        let browser_variants: usize = observation["components"]["schemas"]["BrowserRead"]["oneOf"].as_array().unwrap().iter().map(|variant| variant.get("enum").and_then(Value::as_array).map_or(1, Vec::len)).sum();
        assert_eq!(browser_variants, 13);
        for spec in [&management, &workbench, &observation] {
            let schemas = spec["components"]["schemas"].as_object().unwrap();
            for private in ["AgentReport", "CommandCall", "WorkerInput", "WorkGrant", "InvocationContext", "InspectionRequest", "RuntimeRequest"] {
                assert!(!schemas.contains_key(private), "private input leaked: {private}");
            }
        }
    }
}
