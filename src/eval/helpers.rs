use base64::engine::general_purpose;
use base64::Engine;
use cursive::reexports::log::error;
use itertools::Itertools;
use k8s_openapi::serde_json;
use rhai::Array;

fn is_printable(ch: char) -> bool {
    !matches!(ch, '\u{0000}'..='\u{001F}' | '\u{007F}' | '\u{0080}'..='\u{009F}')
}

pub(crate) fn join(array: Array, delimiter: &str) -> String {
    array.iter().map(|item| item.to_string()).join(delimiter)
}

pub(crate) fn to_yaml(object: rhai::Dynamic) -> String {
    match serde_yaml::to_string(&object) {
        Ok(yaml) => yaml,
        Err(err) => {
            let message = format!("Failed to parse dynamic rhai object {:?}: {err}", object);
            error!("{message}");
            message
        }
    }
}

pub(crate) fn decode_b64(data: &str) -> String {
    static ENGINES: [general_purpose::GeneralPurpose; 4] = [
        general_purpose::STANDARD,
        general_purpose::STANDARD_NO_PAD,
        general_purpose::URL_SAFE,
        general_purpose::URL_SAFE_NO_PAD,
    ];
    for engine in ENGINES.iter() {
        if let Ok(Ok(decoded)) = engine.decode(data).map(String::from_utf8) {
            // we risk converting `aws` to `k{vertical_tab}`
            if !decoded.chars().all(is_printable) {
                return data.to_string();
            }

            return decoded;
        }
    }

    data.to_string()
}

pub(crate) fn pretty_any(raw_string: &str) -> String {
    // skip nasty transformations if they are not needed
    if raw_string.chars().all(|ch| ch.is_ascii_digit()) {
        return raw_string.to_string();
    }

    let decoded = decode_b64(raw_string);

    let parsed = serde_json::from_str::<serde_json::Value>(&decoded)
        .map(|value| serde_json::to_string_pretty(&value));

    if let Ok(Ok(value)) = parsed {
        return value;
    }

    decoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pretty_any() {
        let json = r#"{"a": 1}"#;
        let result = pretty_any(json);
        assert!(result.contains('\n'));

        let b64 = "eyJhIjogNDJ9";
        let result = pretty_any(b64);
        assert!(result.contains("42"));
    }

    #[test]
    fn test_is_printable() {
        assert_eq!(pretty_any("aws"), "aws");
    }
}
