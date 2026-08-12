use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Agent {
    Codex,
    ZCode,
    Claude,
    OpenCode,
}

impl Agent {
    pub const ALL: [Agent; 4] = [Agent::Codex, Agent::ZCode, Agent::Claude, Agent::OpenCode];

    pub fn name(self) -> &'static str {
        match self {
            Agent::Codex => "Codex",
            Agent::ZCode => "ZCode",
            Agent::Claude => "Claude",
            Agent::OpenCode => "OpenCode",
        }
    }
}

impl Serialize for Agent {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for Agent {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Ok(match s.as_str() {
            "Codex" => Agent::Codex,
            "ZCode" => Agent::ZCode,
            "Claude" => Agent::Claude,
            "OpenCode" => Agent::OpenCode,
            other => return Err(serde::de::Error::custom(format!("unknown agent {other}"))),
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AgentPath {
    pub path: String,
    pub detected_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub agents: BTreeMap<Agent, AgentPath>,
}

impl Config {
    pub fn load(path: &Path) -> io::Result<Config> {
        match fs::read(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e),
            Ok(data) => serde_json::from_slice(&data).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("corrupt config: {e}"))
            }),
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let data = serde_json::to_vec_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        let tmp = temp_path(dir);
        fs::write(&tmp, &data)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }
}

fn temp_path(dir: &Path) -> std::path::PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    dir.join(format!(".config-{}-{n}.tmp", std::process::id()))
}
