pub struct ConfigFile<'a> {
    lines: std::str::Lines<'a>,
    current: Option<Vec<&'a str>>,
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
            self.current = self
                .lines
                .next()
                .map(|line| line.split_whitespace().collect());
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
