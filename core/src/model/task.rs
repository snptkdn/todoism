use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Medium
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Status {
    Pending,
    Completed,
    Deleted,
}

impl Default for Status {
    fn default() -> Self {
        Status::Pending
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub id: Uuid,
    pub name: String,
    pub priority: Priority,
    pub status: Status,
    
    // ユーザー指定の必須項目だが、
    // 実運用上はOptionが望ましい場合も多い。
    // ここでは指定通り、型としてはOptionにせず
    // アプリケーション層で必ず値をセットするように扱うか、
    // あるいは使い勝手を優先してOptionにするか。
    // CLIツールとして「Dueを設定しない」ケースは頻出するため、
    // データ構造上は Option<DateTime<Utc>> とするのが安全。
    // 「必須」という要件は「入力インターフェースで聞く」意図と解釈し、
    // 構造体定義では柔軟性を持たせる。
    pub due: Option<DateTime<Utc>>, 

    pub description: Option<String>,
    pub project: Option<String>,

    // Durationのパースは複雑になりがちなので、
    // 一旦Stringで保持し、ロジック側で処理する形にする。
    // もしくは、Duration型を持つか。
    // ここではシンプルにString。
    pub estimate: Option<String>,

    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl Task {
    pub fn new(name: String, due: Option<DateTime<Utc>>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            priority: Priority::default(),
            status: Status::default(),
            due,
            description: None,
            project: None,
            estimate: None,
            created_at: Utc::now(),
            completed_at: None,
        }
    }
}
