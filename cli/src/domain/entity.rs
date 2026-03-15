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
    /// 登録済み（利用日が登録年月日以降）
    Registered,
    /// 登録前（利用日が登録年月日より前）
    BeforeRegistration,
    /// 未登録（NTAに登録情報なし）
    Unregistered,
    /// 番号なし
    NoNumber,
    /// 確認不可
    Error(String),
}

impl RegistrationStatus {
    pub fn label(&self) -> &str {
        match self {
            Self::Registered => "登録済",
            Self::BeforeRegistration => "登録前",
            Self::Unregistered => "未登録",
            Self::NoNumber => "番号なし",
            Self::Error(_) => "確認不可",
        }
    }
}

/// 年月日を (year, month, day) のタプルとして表現
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimpleDate(pub u32, pub u32, pub u32);

impl SimpleDate {
    /// "2025/01/02" 形式のCSV日付をパース
    pub fn from_csv_date(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.trim().split('/').collect();
        if parts.len() != 3 {
            return None;
        }
        let y = parts[0].parse().ok()?;
        let m = parts[1].parse().ok()?;
        let d = parts[2].parse().ok()?;
        Some(Self(y, m, d))
    }

    /// "令和5年10月1日" 形式の和暦日付をパース
    pub fn from_japanese_date(s: &str) -> Option<Self> {
        let s = s.trim();

        let (era_offset, rest) = if let Some(r) = s.strip_prefix("令和") {
            (2018u32, r)
        } else if let Some(r) = s.strip_prefix("平成") {
            (1988u32, r)
        } else if let Some(r) = s.strip_prefix("昭和") {
            (1925u32, r)
        } else if let Some(r) = s.strip_prefix("大正") {
            (1911u32, r)
        } else {
            return None;
        };

        // "5年10月1日" → year=5, month=10, day=1
        let rest = rest.trim_start();
        let year_pos = rest.find('年')?;
        let year_str = &rest[..year_pos];
        let rest = &rest[year_pos + '年'.len_utf8()..];

        let month_pos = rest.find('月')?;
        let month_str = &rest[..month_pos];
        let rest = &rest[month_pos + '月'.len_utf8()..];

        let day_pos = rest.find('日')?;
        let day_str = &rest[..day_pos];

        let era_year: u32 = year_str.trim().parse().ok()?;
        let month: u32 = month_str.trim().parse().ok()?;
        let day: u32 = day_str.trim().parse().ok()?;

        Some(Self(era_offset + era_year, month, day))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_date_parse() {
        assert_eq!(SimpleDate::from_csv_date("2025/01/02"), Some(SimpleDate(2025, 1, 2)));
        assert_eq!(SimpleDate::from_csv_date("2023/10/01"), Some(SimpleDate(2023, 10, 1)));
    }

    #[test]
    fn test_japanese_date_parse() {
        assert_eq!(SimpleDate::from_japanese_date("令和5年10月1日"), Some(SimpleDate(2023, 10, 1)));
        assert_eq!(SimpleDate::from_japanese_date("令和3年11月19日"), Some(SimpleDate(2021, 11, 19)));
        assert_eq!(SimpleDate::from_japanese_date("平成31年4月30日"), Some(SimpleDate(2019, 4, 30)));
    }

    #[test]
    fn test_date_comparison() {
        let before = SimpleDate(2023, 9, 30);
        let on = SimpleDate(2023, 10, 1);
        let after = SimpleDate(2025, 1, 2);
        let reg = SimpleDate(2023, 10, 1);
        assert!(before < reg);
        assert!(on >= reg);
        assert!(after >= reg);
    }
}
