#[derive(Debug, Clone, PartialEq)]
pub enum ConfigToken {
    String(String),
    Float(f32),
    Number(i32),
}

impl From<ConfigToken> for String {
    fn from(value: ConfigToken) -> Self {
        match value {
            ConfigToken::String(s) => s.clone(),
            _ => Default::default(),
        }
    }
}

impl From<ConfigToken> for i32 {
    fn from(value: ConfigToken) -> Self {
        match value {
            ConfigToken::Number(value) => value,
            _ => Default::default(),
        }
    }
}

impl From<ConfigToken> for f32 {
    fn from(value: ConfigToken) -> Self {
        match value {
            ConfigToken::Float(value) => value,
            _ => Default::default(),
        }
    }
}

impl From<ConfigToken> for bool {
    fn from(value: ConfigToken) -> Self {
        match value {
            ConfigToken::String(s) => s.eq_ignore_ascii_case("true"),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Params(Vec<ConfigToken>);

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigLine {
    pub key: String,
    pub params: Params,
}

impl ConfigLine {
    pub fn params(&self) -> &[ConfigToken] {
        &self.params.0
    }

    pub fn param<T: From<ConfigToken> + Default>(&self, index: usize) -> T {
        self.params
            .0
            .get(index)
            .cloned()
            .map(T::from)
            .unwrap_or_default()
    }

    pub fn maybe_param<T: From<ConfigToken> + Default>(&self, index: usize) -> Option<T> {
        self.params.0.get(index).map(|t| T::from(t.clone()))
    }

    pub fn string(&self, index: usize) -> String {
        self.param::<String>(index)
    }
}

fn parse_string(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<String> {
    while chars.peek().is_some_and(|c| c.is_whitespace()) {
        chars.next(); // Skip leading whitespace
    }

    let mut result = String::new();

    match chars.peek()? {
        '"' => {
            chars.next(); // Skip opening quote
            while let Some(&ch) = chars.peek() {
                chars.next();
                if ch == '"' {
                    break;
                }
                result.push(ch);
            }
        }
        _ => {
            while let Some(&ch) = chars.peek() {
                if ch.is_whitespace() {
                    break;
                }
                result.push(ch);
                chars.next();
            }
        }
    }

    Some(result)
}

pub fn parse_line(line: &str) -> Option<ConfigLine> {
    let line = line.trim();
    if line.is_empty() || line.starts_with(';') {
        return None;
    }

    let mut chars = line.chars().peekable();

    let key = parse_string(&mut chars)?;

    let mut params = Vec::new();
    while let Some(param_str) = parse_string(&mut chars) {
        if let Ok(num) = param_str.parse::<i32>() {
            params.push(ConfigToken::Number(num));
        } else if let Ok(num) = param_str.parse::<f32>() {
            params.push(ConfigToken::Float(num));
        } else {
            params.push(ConfigToken::String(param_str));
        }
    }

    Some(ConfigLine {
        key,
        params: Params(params),
    })
}

pub struct ConfigLines {
    lines: Vec<ConfigLine>,
}

impl ConfigLines {
    pub fn parse(s: &str) -> Self {
        Self {
            lines: s.lines().filter_map(parse_line).collect(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &ConfigLine> {
        self.lines.iter()
    }

    pub fn into_iter(self) -> impl Iterator<Item = ConfigLine> {
        self.lines.into_iter()
    }
}
