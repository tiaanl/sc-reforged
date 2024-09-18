pub struct ConfigFile<'a> {
    lines: std::str::Lines<'a>,
    current: Option<Vec<&'a str>>,
}

/// Split the line on whitespace, but also respect strings enclosed in ".
fn split_line(line: &str) -> Vec<&str> {
    let mut result = vec![];

    let mut string_start = None;
    let mut in_string = false;

    for (i, ch) in line.chars().enumerate() {
        match ch {
            '"' => {
                if let Some(start) = string_start {
                    in_string = false;
                    result.push(&line[start..i]);
                    string_start = None;
                } else {
                    in_string = true;
                    string_start = Some(i + 1); // Skip the ".
                }
            }

            ch if ch.is_whitespace() => {
                if !in_string {
                    if let Some(start) = string_start {
                        result.push(&line[start..i]);
                        string_start = None;
                    }
                }
            }

            ch if !ch.is_whitespace() => {
                if string_start.is_none() {
                    string_start = Some(i);
                }
            }

            _ => panic!(),
        }
    }

    if let Some(start) = string_start {
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
