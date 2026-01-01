use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Ask(String),
    Image(String),
    Help,
    Ping,
    Speed,
    Unknown,
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Ask(_) => write!(f, "ask"),
            Command::Image(_) => write!(f, "image"),
            Command::Help => write!(f, "help"),
            Command::Ping => write!(f, "ping"),
            Command::Speed => write!(f, "speed"),
            Command::Unknown => write!(f, "unknown"),
        }
    }
}

impl Command {
    pub fn from_message(text: &str) -> Option<Self> {
        if !text.starts_with('/') {
            return None;
        }

        let parts: Vec<&str> = text.splitn(2, ' ').collect();
        let cmd = parts[0].trim_start_matches('/').to_lowercase();
        let args = parts
            .get(1)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        match cmd.as_str() {
            "ask" => Some(Command::Ask(args)),
            "image" => Some(Command::Image(args)),
            "help" => Some(Command::Help),
            "ping" => Some(Command::Ping),
            "speed" => Some(Command::Speed),
            _ => Some(Command::Unknown),
        }
    }
}
