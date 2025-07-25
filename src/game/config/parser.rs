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

pub fn parse_line(line: &str) -> Option<ConfigLine> {
    let line = line.trim();
    if line.is_empty() || line.starts_with(';') {
        return None;
    }

    let mut chars = line.chars().peekable();

    let mut key = String::new();
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            break;
        }
        key.push(ch);
        chars.next();
    }

    let mut params = Vec::new();
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }
        if ch == '"' {
            chars.next();
            let mut str = String::new();
            while let Some(&str_ch) = chars.peek() {
                chars.next();
                if str_ch == '"' {
                    break;
                }
                str.push(str_ch);
            }
            params.push(ConfigToken::String(str));
        } else {
            let mut str = String::new();
            while let Some(&str_ch) = chars.peek() {
                if str_ch.is_whitespace() {
                    break;
                }
                str.push(str_ch);
                chars.next();
            }

            if let Ok(num) = str.parse::<i32>() {
                params.push(ConfigToken::Number(num));
            } else if let Ok(num) = str.parse::<f32>() {
                params.push(ConfigToken::Float(num));
            } else {
                params.push(ConfigToken::String(str));
            }
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
