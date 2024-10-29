use miette::{miette, LabeledSpan};

/// The output value from the parser
#[derive(PartialEq, Debug, Clone)]
pub enum Value {
    String(String),
    Integer(i32),
    Array(Vec<Value>),
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Integer(value)
    }
}

pub struct RedisParser<'a> {
    cursor: &'a [u8],
    full: &'a [u8],
}

impl<'a> RedisParser<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            cursor: input,
            full: input,
        }
    }

    /// Parse the input as a Redis encoded [`Value`].
    fn parse_value(&mut self) -> miette::Result<Value> {
        match self.cursor.first().ok_or_else(|| miette!("empty input"))? {
            // Integer
            b':' => self.parse_int().map(Into::into),
            // String
            b'$' => self.parse_string().map(Into::into),
            _ => Err(miette!(
                labels = vec![LabeledSpan::at_offset(
                    self.full.len() - self.cursor.len(),
                    "here"
                )],
                "failed to parse input as value",
            )
            .with_source_code(self.full.to_vec())),
        }
    }

    /// Parses the input as a Redis encoded integer.
    /// Returns the parsed integer and moves the cursor.
    fn parse_int(&mut self) -> miette::Result<i32> {
        let input = self.cursor;
        // Verify the length is correct for the rest of the parsing
        let end = input
            .iter()
            .position(|b| b == &b'\n')
            .ok_or_else(|| miette!("failed to find \\n terminator"))?;

        // Extract offset and sign
        let sub_bytes = &input[..end];
        let sign = sub_bytes[1];
        let offset = if sign == b'+' || sign == b'-' { 1 } else { 0 };
        let sign = if sign == b'-' { -1 } else { 1 };

        // Parse the value
        let value = sub_bytes
            .iter()
            .position(|b| b == &b'\r')
            .and_then(|pos| sub_bytes.get(1 + offset..pos))
            .and_then(|v| std::str::from_utf8(v).ok())
            .and_then(|v| v.parse::<i32>().ok())
            .ok_or_else(|| {
                miette!(
                    labels = vec![LabeledSpan::at_offset(
                        self.full.len() - self.cursor.len() - offset,
                        "here"
                    )],
                    "failed to parse to int",
                )
                .with_source_code(self.full.to_vec())
            })?;

        self.cursor = &self.cursor[..end + 1];
        Ok(sign * value)
    }

    /// Parses the input as a Redis encoded string.
    /// Returns the parsed String and moves the cursor.
    fn parse_string(&mut self) -> miette::Result<String> {
        let input = self.cursor;

        // Get the end of the length defining bytes
        let end_length = input
            .iter()
            .position(|b| b == &b'\n')
            .ok_or_else(|| miette!("failed to find first \\n terminator"))?;
        let end_string = input
            .get(end_length + 1..)
            .and_then(|bytes| bytes.iter().position(|b| b == &b'\r'))
            .ok_or_else(|| miette!("failed to find second \\n terminator"))?;

        // Extract the string
        let s = input
            .get(end_length + 1..end_length + 1 + end_string)
            .and_then(|s| String::from_utf8(s.to_vec()).ok())
            .ok_or_else(|| {
                miette!(
                    labels = vec![LabeledSpan::at_offset(
                        self.full.len() - self.cursor.len() - end_length - 1,
                        "here"
                    )],
                    "failed to parse to bytes to utf8",
                )
                .with_source_code(self.full.to_vec())
            })?;

        self.cursor = &self.cursor[..end_length + 1 + end_string];
        Ok(s)
    }
}

impl<'a> Iterator for RedisParser<'a> {
    type Item = miette::Result<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        let bytes = self.cursor;
        match bytes.first()? {
            // Array
            // TODO: move this to parse_array.
            // TODO: remove parse_value and just use parse_value, parse_string and parse_array here.
            b'*' => {
                let end_length = bytes.iter().position(|b| b == &b'\r')?;
                let length = bytes
                    .get(1..end_length)
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    .and_then(|s| s.parse::<usize>().ok())?;

                // Advance cursor to the start of the array
                self.cursor = &self.cursor[..end_length + 2];

                let mut output = Vec::with_capacity(length);
                for _ in 0..length {
                    let value = self.parse_value();
                    if value.is_err() {
                        return Some(value);
                    }
                    output.push(value.expect("not error"));
                }
                Some(Ok(Value::Array(output)))
            }
            _ => Some(self.parse_value()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_positive_integer() -> miette::Result<()> {
        // Given
        let input = b":100\r\n";

        // When
        let mut parser = RedisParser::new(&input[..]);

        // Then
        let parsed = parser.next().unwrap()?;

        assert_eq!(parsed, Value::Integer(100));
        Ok(())
    }

    #[test]
    fn test_parse_negative_integer() -> miette::Result<()> {
        // Given
        let input = b":-100\r\n";

        // When
        let mut parser = RedisParser::new(&input[..]);

        // Then
        let parsed = parser.next().unwrap()?;

        assert_eq!(parsed, Value::Integer(-100));
        Ok(())
    }

    #[test]
    fn test_parse_empty_string() -> miette::Result<()> {
        // Given
        let input = b"$0\r\n\r\n";

        // When
        let mut parser = RedisParser::new(&input[..]);

        // Then
        let parsed = parser.next().unwrap()?;

        assert_eq!(parsed, Value::String(String::from("")));
        Ok(())
    }

    #[test]
    fn test_parse_string() -> miette::Result<()> {
        // Given
        let input = b"$5\r\nhello\r\n";

        // When
        let mut parser = RedisParser::new(&input[..]);

        // Then
        let parsed = parser.next().unwrap()?;

        assert_eq!(parsed, Value::String(String::from("hello")));
        Ok(())
    }

    #[test]
    fn test_parse_empty_array() -> miette::Result<()> {
        // Given
        let input = b"*0\r\n";

        // When
        let mut parser = RedisParser::new(&input[..]);

        // Then
        let parsed = parser.next().unwrap()?;

        assert_eq!(parsed, Value::Array(Vec::new()));
        Ok(())
    }

    #[test]
    fn test_parse_array() -> miette::Result<()> {
        // Given
        let input = b"*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n";

        // When
        let mut parser = RedisParser::new(&input[..]);

        // Then
        let parsed = parser.next().unwrap()?;

        assert_eq!(
            parsed,
            Value::Array(vec![
                Value::String("hello".into()),
                Value::String("world".into())
            ])
        );
        Ok(())
    }
}
