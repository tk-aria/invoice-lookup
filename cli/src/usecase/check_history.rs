use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::domain::entity::{HistoryRow, RegistrationStatus, SimpleDate};
use crate::domain::repository::InvoiceRepository;

/// CSVファイルの全行に対してインボイス登録状況を確認するユースケース
///
/// 登録済みT番号の場合、CSV行の利用日と登録年月日を比較して:
/// - 利用日 >= 登録年月日 → 登録済
/// - 利用日 <  登録年月日 → 登録前
pub struct CheckHistoryUseCase {
    repo: Arc<dyn InvoiceRepository>,
}

/// T番号ごとの検索結果を保持
struct TnumInfo {
    registered: bool,
    registration_date: Option<SimpleDate>,
}

impl CheckHistoryUseCase {
    pub fn new(repo: Arc<dyn InvoiceRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, csv_content: &str) -> CheckHistoryResult {
        let lines: Vec<&str> = csv_content.lines().collect();
        if lines.is_empty() {
            return CheckHistoryResult {
                header: String::new(),
                rows: vec![],
            };
        }

        let header = lines[0].to_string();
        let data_lines = &lines[1..];

        // ユニークなT番号を収集
        let mut unique_tnums: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut row_tnums: Vec<Option<String>> = Vec::new();

        for line in data_lines {
            let cols = parse_csv_line(line);
            let tnum = if cols.len() > 4 {
                normalize_tnum(cols[4].trim())
            } else {
                None
            };
            if let Some(ref t) = tnum {
                if seen.insert(t.clone()) {
                    unique_tnums.push(t.clone());
                }
            }
            row_tnums.push(tnum);
        }

        // バッチ検索
        let results = self.repo.find_batch(&unique_tnums).await;
        let mut info_map: HashMap<String, TnumInfo> = HashMap::new();
        for (tnum, result) in unique_tnums.iter().zip(results.into_iter()) {
            if let Ok(info) = result {
                let reg_date = SimpleDate::from_japanese_date(&info.registration_date);
                info_map.insert(
                    tnum.clone(),
                    TnumInfo {
                        registered: info.registered,
                        registration_date: reg_date,
                    },
                );
            }
        }

        // 各行にステータスを割り当て（日付比較あり）
        let rows: Vec<HistoryRow> = data_lines
            .iter()
            .zip(row_tnums.iter())
            .enumerate()
            .map(|(i, (line, tnum))| {
                let cols = parse_csv_line(line);
                let csv_date = if !cols.is_empty() {
                    SimpleDate::from_csv_date(cols[0].trim())
                } else {
                    None
                };

                let status = match tnum {
                    Some(t) => match info_map.get(t) {
                        Some(info) if !info.registered => RegistrationStatus::Unregistered,
                        Some(info) => {
                            // 登録済み: 日付比較
                            match (csv_date, info.registration_date) {
                                (Some(use_date), Some(reg_date)) if use_date < reg_date => {
                                    RegistrationStatus::BeforeRegistration
                                }
                                _ => RegistrationStatus::Registered,
                            }
                        }
                        None => RegistrationStatus::Error("lookup failed".to_string()),
                    },
                    None => RegistrationStatus::NoNumber,
                };
                HistoryRow {
                    line_number: i + 2,
                    raw_line: line.to_string(),
                    t_number: tnum.clone(),
                    status,
                }
            })
            .collect();

        CheckHistoryResult { header, rows }
    }
}

pub struct CheckHistoryResult {
    pub header: String,
    pub rows: Vec<HistoryRow>,
}

impl CheckHistoryResult {
    pub fn to_csv(&self) -> String {
        let mut out = format!("{},登録状況\n", self.header);
        for row in &self.rows {
            out.push_str(&format!("{},{}\n", row.raw_line, row.status.label()));
        }
        out
    }

    pub fn summary(&self) -> HistorySummary {
        let mut s = HistorySummary::default();
        for row in &self.rows {
            match &row.status {
                RegistrationStatus::Registered => s.registered += 1,
                RegistrationStatus::BeforeRegistration => {
                    s.before_registration += 1;
                    s.before_registration_entries.push(format!(
                        "{} (行{})",
                        row.t_number.as_deref().unwrap_or("?"),
                        row.line_number
                    ));
                }
                RegistrationStatus::Unregistered => {
                    s.unregistered += 1;
                    s.unregistered_numbers
                        .push(row.t_number.clone().unwrap_or_default());
                }
                RegistrationStatus::NoNumber => s.no_number += 1,
                RegistrationStatus::Error(_) => s.errors += 1,
            }
        }
        s.total = self.rows.len();
        s
    }
}

#[derive(Debug, Default)]
pub struct HistorySummary {
    pub total: usize,
    pub registered: usize,
    pub before_registration: usize,
    pub unregistered: usize,
    pub no_number: usize,
    pub errors: usize,
    pub unregistered_numbers: Vec<String>,
    pub before_registration_entries: Vec<String>,
}

fn normalize_tnum(raw: &str) -> Option<String> {
    if raw.is_empty() || raw == "なし" {
        return None;
    }
    let digits = raw.strip_prefix('T').unwrap_or(raw);
    if digits.len() == 13 && digits.chars().all(|c| c.is_ascii_digit()) {
        Some(format!("T{}", digits))
    } else {
        None
    }
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in line.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
        } else if ch == ',' && !in_quotes {
            fields.push(current.clone());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}
