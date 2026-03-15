use serde::Serialize;

/// インボイス登録情報（ドメインエンティティ）
#[derive(Debug, Clone, Serialize)]
pub struct InvoiceRegistration {
    pub t_number: String,
    pub name: String,
    pub registration_date: String,
    pub address: String,
    pub last_updated: String,
    pub registered: bool,
}

/// CSV行とインボイス登録状況を結合したエンティティ
#[derive(Debug, Clone)]
pub struct HistoryRow {
    pub line_number: usize,
    pub raw_line: String,
    pub t_number: Option<String>,
    pub status: RegistrationStatus,
}

#[derive(Debug, Clone, Serialize)]
pub enum RegistrationStatus {
    Registered,
    Unregistered,
    NoNumber,
    Error(String),
}

impl RegistrationStatus {
    pub fn label(&self) -> &str {
        match self {
            Self::Registered => "登録済",
            Self::Unregistered => "未登録",
            Self::NoNumber => "番号なし",
            Self::Error(_) => "確認不可",
        }
    }
}
