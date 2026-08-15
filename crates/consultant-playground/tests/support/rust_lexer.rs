//! Minimal Rust tokenizer used to compare production-source shape against an
//! exact expected token stream, independent of whitespace/comment layout.

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RustToken {
    Ident(String),
    Punct(char),
    Literal(String),
}

pub(crate) struct RustLexer<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> RustLexer<'a> {
    pub(crate) fn lex(source: &'a str) -> Result<Vec<RustToken>, String> {
        let mut lexer = Self {
            bytes: source.as_bytes(),
            cursor: 0,
        };
        let mut tokens = Vec::new();
        while lexer.cursor < lexer.bytes.len() {
            let byte = lexer.bytes[lexer.cursor];
            if byte.is_ascii_whitespace() {
                lexer.cursor += 1;
            } else if lexer.starts_with(b"///")
                || lexer.starts_with(b"//!")
                || lexer.starts_with(b"/**")
                || lexer.starts_with(b"/*!")
            {
                return Err("Rust doc comments are attributes outside the Task 1 grammar".into());
            } else if lexer.starts_with(b"//") {
                lexer.skip_line_comment();
            } else if lexer.starts_with(b"/*") {
                lexer.skip_block_comment()?;
            } else if lexer.raw_string_prefix().is_some() {
                let start = lexer.cursor;
                lexer.skip_raw_string()?;
                tokens.push(RustToken::Literal(lexer.source_slice(start)?));
            } else if byte == b'"' {
                let start = lexer.cursor;
                lexer.skip_quoted(b'"')?;
                tokens.push(RustToken::Literal(lexer.source_slice(start)?));
            } else if lexer.starts_with(b"r#")
                && lexer
                    .bytes
                    .get(lexer.cursor + 2)
                    .is_some_and(|byte| is_ident_start(*byte))
            {
                lexer.cursor += 2;
                tokens.push(RustToken::Ident(lexer.take_identifier()?));
            } else if is_ident_start(byte) {
                tokens.push(RustToken::Ident(lexer.take_identifier()?));
            } else if byte.is_ascii_digit() {
                let start = lexer.cursor;
                lexer.skip_number();
                tokens.push(RustToken::Literal(lexer.source_slice(start)?));
            } else if byte.is_ascii_punctuation() {
                lexer.cursor += 1;
                tokens.push(RustToken::Punct(char::from(byte)));
            } else {
                return Err(format!(
                    "unsupported non-ASCII Rust token at byte {}",
                    lexer.cursor
                ));
            }
        }
        Ok(tokens)
    }

    fn starts_with(&self, value: &[u8]) -> bool {
        self.bytes[self.cursor..].starts_with(value)
    }

    fn take_identifier(&mut self) -> Result<String, String> {
        let start = self.cursor;
        if !self
            .bytes
            .get(self.cursor)
            .is_some_and(|byte| is_ident_start(*byte))
        {
            return Err("identifier has no valid first byte".into());
        }
        self.cursor += 1;
        while self
            .bytes
            .get(self.cursor)
            .is_some_and(|byte| is_ident_continue(*byte))
        {
            self.cursor += 1;
        }
        String::from_utf8(self.bytes[start..self.cursor].to_vec())
            .map_err(|error| format!("identifier is not UTF-8: {error}"))
    }

    fn source_slice(&self, start: usize) -> Result<String, String> {
        String::from_utf8(self.bytes[start..self.cursor].to_vec())
            .map_err(|error| format!("Rust token is not UTF-8: {error}"))
    }

    fn skip_line_comment(&mut self) {
        self.cursor += 2;
        while self.cursor < self.bytes.len() && self.bytes[self.cursor] != b'\n' {
            self.cursor += 1;
        }
    }

    fn skip_block_comment(&mut self) -> Result<(), String> {
        self.cursor += 2;
        let mut depth = 1_u32;
        while self.cursor < self.bytes.len() {
            if self.starts_with(b"/*") {
                depth += 1;
                self.cursor += 2;
            } else if self.starts_with(b"*/") {
                depth -= 1;
                self.cursor += 2;
                if depth == 0 {
                    return Ok(());
                }
            } else {
                self.cursor += 1;
            }
        }
        Err("unterminated Rust block comment".into())
    }

    fn raw_string_prefix(&self) -> Option<(usize, usize)> {
        let prefix = if self.starts_with(b"br") { 2 } else { 1 };
        if self.bytes.get(self.cursor) != Some(&b'r') && !self.starts_with(b"br") {
            return None;
        }
        let mut cursor = self.cursor + prefix;
        let mut hashes = 0;
        while self.bytes.get(cursor) == Some(&b'#') {
            hashes += 1;
            cursor += 1;
        }
        (self.bytes.get(cursor) == Some(&b'"')).then_some((cursor, hashes))
    }

    fn skip_raw_string(&mut self) -> Result<(), String> {
        let (quote, hashes) = self
            .raw_string_prefix()
            .ok_or_else(|| "invalid raw string prefix".to_string())?;
        self.cursor = quote + 1;
        while self.cursor < self.bytes.len() {
            if self.bytes[self.cursor] == b'"'
                && self.bytes.get(self.cursor + 1..self.cursor + 1 + hashes)
                    == Some(&vec![b'#'; hashes][..])
            {
                self.cursor += 1 + hashes;
                return Ok(());
            }
            self.cursor += 1;
        }
        Err("unterminated Rust raw string".into())
    }

    fn skip_quoted(&mut self, quote: u8) -> Result<(), String> {
        self.cursor += 1;
        while self.cursor < self.bytes.len() {
            match self.bytes[self.cursor] {
                b'\\' => self.cursor = (self.cursor + 2).min(self.bytes.len()),
                byte if byte == quote => {
                    self.cursor += 1;
                    return Ok(());
                }
                _ => self.cursor += 1,
            }
        }
        Err("unterminated Rust quoted literal".into())
    }

    fn skip_number(&mut self) {
        while self.bytes.get(self.cursor).is_some_and(|byte| {
            byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'.' | b'+' | b'-')
        }) {
            self.cursor += 1;
        }
    }
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    is_ident_start(byte) || byte.is_ascii_digit()
}
