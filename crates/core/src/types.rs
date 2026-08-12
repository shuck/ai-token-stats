#[derive(Debug, Clone, Default)]
pub struct Usage {
    pub input: i64,
    pub cached: i64,
    pub cache_write: i64,
    pub output: i64,
    pub reasoning: i64,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct Record {
    pub thread_id: String,
    pub agent: String,
    pub model: String,
    pub key: String,
    pub path: String,
    pub ts: i64,
    pub date: String,
    pub usage: Usage,
    pub context_window: Option<i64>,
}
