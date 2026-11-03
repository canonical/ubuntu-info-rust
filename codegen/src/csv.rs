//! A minimal CSV reader covering the RFC 4180 subset used by `distro-info-data`.
//!
//! Fields are separated by commas and records by newlines. Double-quoted fields
//! may contain commas and newlines and escape quotes by doubling them. Blank
//! lines are skipped so that trailing newlines do not produce empty records.

/// Parses CSV text into records of fields.
pub fn parse(input: &str) -> Result<Vec<Vec<String>>, String> {
    // Tolerate a UTF-8 byte order mark, which some editors and proxies add.
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);

    let mut records: Vec<Vec<String>> = Vec::new();
    let mut record: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut quote_just_closed = false;
    let mut at_field_start = true;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_quotes {
            match ch {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => {
                    in_quotes = false;
                    quote_just_closed = true;
                }
                _ => field.push(ch),
            }
            continue;
        }

        match ch {
            ',' => {
                record.push(std::mem::take(&mut field));
                at_field_start = true;
                quote_just_closed = false;
            }
            '\n' => {
                record.push(std::mem::take(&mut field));
                if record.len() > 1 || !record[0].is_empty() {
                    records.push(std::mem::take(&mut record));
                }
                record.clear();
                at_field_start = true;
                quote_just_closed = false;
            }
            '\r' if chars.peek() == Some(&'\n') => {}
            '"' if at_field_start => {
                in_quotes = true;
                at_field_start = false;
            }
            '"' => return Err("unexpected quote inside an unquoted CSV field".to_owned()),
            _ => {
                if quote_just_closed {
                    return Err("unexpected text after a closing CSV quote".to_owned());
                }
                field.push(ch);
                at_field_start = false;
            }
        }
    }

    if in_quotes {
        return Err("unterminated quoted CSV field".to_owned());
    }

    record.push(field);
    if record.len() > 1 || !record[0].is_empty() {
        records.push(record);
    }

    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_records() {
        let records = parse("a,b,c\n1,2,3\n").unwrap();
        assert_eq!(
            records,
            [
                vec!["a".to_owned(), "b".to_owned(), "c".to_owned()],
                vec!["1".to_owned(), "2".to_owned(), "3".to_owned()],
            ]
        );
    }

    #[test]
    fn handles_missing_trailing_newline() {
        let records = parse("a,b\n1,2").unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[1], ["1".to_owned(), "2".to_owned()]);
    }

    #[test]
    fn skips_blank_lines() {
        let records = parse("a,b\n\n1,2\n\n").unwrap();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn handles_crlf_line_endings() {
        let records = parse("a,b\r\n1,2\r\n").unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[1], ["1".to_owned(), "2".to_owned()]);
    }

    #[test]
    fn parses_quoted_fields() {
        let records = parse("\"a,b\",\"c\"\"d\"\n\"multi\nline\",2\n").unwrap();
        assert_eq!(records[0], ["a,b".to_owned(), "c\"d".to_owned()]);
        assert_eq!(records[1], ["multi\nline".to_owned(), "2".to_owned()]);
    }

    #[test]
    fn rejects_malformed_quotes() {
        assert!(parse("\"unterminated\n").is_err());
        assert!(parse("before\"after\n").is_err());
        assert!(parse("\"closed\"text\n").is_err());
    }

    #[test]
    fn parses_empty_input() {
        assert_eq!(parse("").unwrap(), Vec::<Vec<String>>::new());
    }

    #[test]
    fn strips_a_utf8_byte_order_mark() {
        let records = parse("\u{feff}a,b\n").unwrap();
        assert_eq!(records[0], ["a".to_owned(), "b".to_owned()]);
    }
}
