//! Minimal JSON parser used to read `cargo metadata` output without pulling
//! a JSON dependency into the boundary-test fixtures.

use std::collections::BTreeMap;

#[derive(Debug)]
pub(crate) enum JsonValue {
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
    Scalar,
}

pub(crate) struct JsonParser<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> JsonParser<'a> {
    pub(crate) fn parse(bytes: &'a [u8]) -> Result<JsonValue, String> {
        let mut parser = Self { bytes, cursor: 0 };
        let value = parser.parse_value()?;
        parser.skip_whitespace();
        if parser.cursor != parser.bytes.len() {
            return Err(format!(
                "unexpected trailing JSON at byte {}",
                parser.cursor
            ));
        }
        Ok(value)
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        match self.bytes.get(self.cursor).copied() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b't') => self.parse_keyword(b"true"),
            Some(b'f') => self.parse_keyword(b"false"),
            Some(b'n') => self.parse_keyword(b"null"),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            Some(byte) => Err(format!(
                "unexpected JSON byte `{}` at {}",
                char::from(byte),
                self.cursor
            )),
            None => Err("unexpected end of JSON".into()),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.cursor += 1;
        let mut values = BTreeMap::new();
        self.skip_whitespace();
        if self.take(b'}') {
            return Ok(JsonValue::Object(values));
        }
        loop {
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(b':')?;
            let value = self.parse_value()?;
            if values.insert(key.clone(), value).is_some() {
                return Err(format!("duplicate JSON object key `{key}`"));
            }
            self.skip_whitespace();
            if self.take(b'}') {
                break;
            }
            self.expect(b',')?;
            self.skip_whitespace();
        }
        Ok(JsonValue::Object(values))
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.cursor += 1;
        let mut values = Vec::new();
        self.skip_whitespace();
        if self.take(b']') {
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.parse_value()?);
            self.skip_whitespace();
            if self.take(b']') {
                break;
            }
            self.expect(b',')?;
        }
        Ok(JsonValue::Array(values))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut value = Vec::new();
        while let Some(byte) = self.bytes.get(self.cursor).copied() {
            self.cursor += 1;
            match byte {
                b'"' => {
                    return String::from_utf8(value)
                        .map_err(|error| format!("JSON string is not UTF-8: {error}"));
                }
                b'\\' => self.parse_string_escape(&mut value)?,
                0x00..=0x1f => return Err("JSON string contains a control byte".into()),
                _ => value.push(byte),
            }
        }
        Err("unterminated JSON string".into())
    }

    fn parse_string_escape(&mut self, value: &mut Vec<u8>) -> Result<(), String> {
        let escaped = self
            .bytes
            .get(self.cursor)
            .copied()
            .ok_or_else(|| "unterminated JSON escape".to_string())?;
        self.cursor += 1;
        match escaped {
            b'"' | b'\\' | b'/' => value.push(escaped),
            b'b' => value.push(0x08),
            b'f' => value.push(0x0c),
            b'n' => value.push(b'\n'),
            b'r' => value.push(b'\r'),
            b't' => value.push(b'\t'),
            b'u' => {
                let end = self.cursor + 4;
                let digits = self
                    .bytes
                    .get(self.cursor..end)
                    .ok_or_else(|| "short JSON Unicode escape".to_string())?;
                let digits = std::str::from_utf8(digits)
                    .map_err(|error| format!("invalid JSON Unicode escape: {error}"))?;
                let code = u32::from_str_radix(digits, 16)
                    .map_err(|error| format!("invalid JSON Unicode escape: {error}"))?;
                let character = char::from_u32(code)
                    .ok_or_else(|| "unsupported JSON surrogate escape".to_string())?;
                let mut encoded = [0_u8; 4];
                value.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
                self.cursor = end;
            }
            _ => return Err("unknown JSON string escape".into()),
        }
        Ok(())
    }

    fn parse_keyword(&mut self, keyword: &[u8]) -> Result<JsonValue, String> {
        if !self.bytes[self.cursor..].starts_with(keyword) {
            return Err(format!("invalid JSON keyword at byte {}", self.cursor));
        }
        self.cursor += keyword.len();
        Ok(JsonValue::Scalar)
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let start = self.cursor;
        while self.bytes.get(self.cursor).is_some_and(|byte| {
            byte.is_ascii_digit() || matches!(*byte, b'-' | b'+' | b'.' | b'e' | b'E')
        }) {
            self.cursor += 1;
        }
        if self.cursor == start {
            return Err("empty JSON number".into());
        }
        Ok(JsonValue::Scalar)
    }

    fn skip_whitespace(&mut self) {
        while self
            .bytes
            .get(self.cursor)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            self.cursor += 1;
        }
    }

    fn take(&mut self, expected: u8) -> bool {
        if self.bytes.get(self.cursor) == Some(&expected) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), String> {
        if self.take(expected) {
            Ok(())
        } else {
            Err(format!(
                "expected JSON `{}` at byte {}",
                char::from(expected),
                self.cursor
            ))
        }
    }
}

pub(crate) fn json_object<'a>(
    value: &'a JsonValue,
    label: &str,
) -> Result<&'a BTreeMap<String, JsonValue>, String> {
    match value {
        JsonValue::Object(value) => Ok(value),
        _ => Err(format!("Cargo metadata `{label}` is not an object")),
    }
}

pub(crate) fn json_array<'a>(value: &'a JsonValue, label: &str) -> Result<&'a [JsonValue], String> {
    match value {
        JsonValue::Array(value) => Ok(value),
        _ => Err(format!("Cargo metadata `{label}` is not an array")),
    }
}

pub(crate) fn json_string<'a>(value: &'a JsonValue, label: &str) -> Result<&'a str, String> {
    match value {
        JsonValue::String(value) => Ok(value),
        _ => Err(format!("Cargo metadata `{label}` is not a string")),
    }
}

pub(crate) fn json_field<'a>(value: &'a JsonValue, field: &str) -> Result<&'a JsonValue, String> {
    json_object(value, "object")?
        .get(field)
        .ok_or_else(|| format!("Cargo metadata omitted `{field}`"))
}

pub(crate) fn expect_json_string(
    value: &JsonValue,
    field: &str,
    expected: &str,
) -> Result<(), String> {
    let actual = json_string(json_field(value, field)?, field)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "Cargo metadata `{field}` is `{actual}`, expected `{expected}`"
        ))
    }
}
