pub struct ConfigFile<'a> {
    lines: std::str::Lines<'a>,
    current: Option<Vec<&'a str>>,
}

/// Splits a line into tokens. Quoted strings are treated as a single token (without quotes).
/// Non-whitespace sequences are considered tokens outside of strings.
fn split_line(line: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut in_string = false;
    let mut token_start: Option<usize> = None;

    for (i, ch) in line.char_indices() {
        match ch {
            '"' => {
                if in_string {
                    // End of quoted string
                    if let Some(start) = token_start {
                        result.push(&line[start..i]);
                        token_start = None;
                    }
                    in_string = false;
                } else {
                    // Start of quoted string (skip quote)
                    in_string = true;
                    token_start = Some(i + 1);
                }
            }

            ch if ch.is_whitespace() => {
                if !in_string {
                    if let Some(start) = token_start {
                        result.push(&line[start..i]);
                        token_start = None;
                    }
                }
            }

            _ => {
                if token_start.is_none() {
                    token_start = Some(i);
                }
            }
        }
    }

    // Handle final token
    if let Some(start) = token_start {
        result.push(&line[start..]);
    }

    result
}

impl<'a> ConfigFile<'a> {
    pub fn new(data: &'a str) -> Self {
        let mut s = Self {
            lines: data.lines(),
            current: None,
        };

        s.next();

        s
    }

    pub fn next(&mut self) {
        loop {
            self.current = self.lines.next().map(split_line);
            if let Some(ref current) = self.current {
                if current.is_empty() || current[0].starts_with(';') {
                    continue;
                }
            }
            break;
        }
    }

    pub fn current(&self) -> Option<&Vec<&'a str>> {
        self.current.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        assert_eq!(split_line("one"), vec!["one"]);
        assert_eq!(split_line("one two"), vec!["one", "two"]);
        assert_eq!(split_line("one two three"), vec!["one", "two", "three"]);
        assert_eq!(split_line("one    two"), vec!["one", "two"]);
        assert_eq!(split_line("one\t\ttwo"), vec!["one", "two"]);
        assert_eq!(split_line("one \"two three\""), vec!["one", "two three"]);
    }
}
