use std::sync::OnceLock;

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceInfo {
    /// 氏名又は名称
    pub name: String,
    /// 登録年月日
    pub registration_date: String,
    /// 本店又は主たる事務所の所在地
    pub address: String,
    /// 最終更新年月日
    pub last_updated: String,
    /// インボイス登録済みか未登録か
    pub registered: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum InvoiceLookupError {
    #[error("Invalid T-number format: {0}")]
    InvalidFormat(String),
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Failed to parse HTML response")]
    ParseError,
}

const BASE_URL: &str = "https://www.invoice-kohyo.nta.go.jp/regno-search/detail";

/// Cached CSS selectors (compiled once, reused across all calls)
struct CachedSelectors {
    label: Selector,
    data: Selector,
}

fn cached_selectors() -> &'static CachedSelectors {
    static SELECTORS: OnceLock<CachedSelectors> = OnceLock::new();
    SELECTORS.get_or_init(|| CachedSelectors {
        label: Selector::parse("h3.itemlabel").unwrap(),
        data: Selector::parse("p.itemdata").unwrap(),
    })
}

/// HTTP client with connection pooling for repeated lookups.
///
/// Reuses TCP connections and TLS sessions, reducing latency from ~220ms to ~50-80ms
/// on subsequent requests.
pub struct InvoiceLookupClient {
    client: reqwest::Client,
}

impl InvoiceLookupClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .pool_max_idle_per_host(10)
            .build()
            .expect("failed to build reqwest client");
        Self { client }
    }

    /// T番号からインボイス登録情報を取得する
    pub async fn lookup(&self, t_number: &str) -> Result<InvoiceInfo, InvoiceLookupError> {
        let digits = parse_t_number(t_number)?;
        let url = format!("{}?selRegNo={}", BASE_URL, digits);
        let html = self.client.get(&url).send().await?.text().await?;
        parse_invoice_html(&html)
    }

    /// 複数のT番号を並行して検索する
    pub async fn lookup_batch(
        &self,
        t_numbers: &[&str],
    ) -> Vec<Result<InvoiceInfo, InvoiceLookupError>> {
        let futs: Vec<_> = t_numbers.iter().map(|t| self.lookup(t)).collect();
        futures::future::join_all(futs).await
    }
}

impl Default for InvoiceLookupClient {
    fn default() -> Self {
        Self::new()
    }
}

/// T番号からインボイス登録情報を取得する (簡易API、毎回新しいClientを作成)
///
/// 繰り返し呼ぶ場合は `InvoiceLookupClient` を使用してください。
pub async fn lookup(t_number: &str) -> Result<InvoiceInfo, InvoiceLookupError> {
    let client = InvoiceLookupClient::new();
    client.lookup(t_number).await
}

fn parse_t_number(t_number: &str) -> Result<&str, InvoiceLookupError> {
    let digits = t_number.strip_prefix('T').unwrap_or(t_number);

    if digits.len() != 13 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(InvoiceLookupError::InvalidFormat(format!(
            "Expected T + 13 digits, got: {}",
            t_number
        )));
    }

    Ok(digits)
}

fn parse_invoice_html(html: &str) -> Result<InvoiceInfo, InvoiceLookupError> {
    // 未登録チェック: DOM解析前に文字列検索で短絡
    if html.contains("検索対象の登録番号は存在しません") {
        return Ok(InvoiceInfo {
            name: String::new(),
            registration_date: String::new(),
            address: String::new(),
            last_updated: String::new(),
            registered: false,
        });
    }

    let document = Html::parse_document(html);
    let sels = cached_selectors();

    let labels: Vec<String> = document
        .select(&sels.label)
        .map(|el| el.text().collect::<String>().trim().to_string())
        .collect();
    let values: Vec<String> = document
        .select(&sels.data)
        .map(|el| el.text().collect::<String>().trim().to_string())
        .collect();

    let mut name = String::new();
    let mut registration_date = String::new();
    let mut address = String::new();
    let mut last_updated = String::new();

    for (label, value) in labels.iter().zip(values.iter()) {
        match label.as_str() {
            "氏名又は名称" => name = value.clone(),
            "登録年月日" => registration_date = value.clone(),
            "本店又は主たる事務所の所在地" => address = value.clone(),
            "最終更新年月日" => last_updated = value.clone(),
            _ => {}
        }
    }

    Ok(InvoiceInfo {
        name,
        registration_date,
        address,
        last_updated,
        registered: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_t_number_valid() {
        assert_eq!(parse_t_number("T8013201004026").unwrap(), "8013201004026");
    }

    #[test]
    fn test_parse_t_number_without_prefix() {
        assert_eq!(parse_t_number("8013201004026").unwrap(), "8013201004026");
    }

    #[test]
    fn test_parse_t_number_invalid_length() {
        assert!(parse_t_number("T123").is_err());
    }

    #[test]
    fn test_parse_t_number_invalid_chars() {
        assert!(parse_t_number("T801320100402A").is_err());
    }

    #[test]
    fn test_parse_unregistered_html() {
        let html = r#"<html><body><li>検索対象の登録番号は存在しません。内容をお確かめのうえ、入力してください。</li></body></html>"#;
        let info = parse_invoice_html(html).unwrap();
        assert!(!info.registered);
        assert!(info.name.is_empty());
    }

    #[test]
    fn test_parse_registered_html() {
        let html = r#"
        <html><body>
            <h3 class="itemlabel">登録番号</h3>
            <p class="itemdata">T8013201004026</p>
            <h3 class="nmTsuushou_label itemlabel">氏名又は名称</h3>
            <p class="itemdata">株式会社東急ストア</p>
            <h3 class="itemlabel">登録年月日</h3>
            <p class="itemdata">令和5年10月1日</p>
            <h3 class="hontenaddr_label itemlabel">本店又は主たる事務所の所在地</h3>
            <p class="itemdata">東京都目黒区上目黒１丁目２１番１２号</p>
            <h3 class="itemlabel">最終更新年月日</h3>
            <p class="itemdata latestdate">令和3年11月19日</p>
        </body></html>"#;
        let info = parse_invoice_html(html).unwrap();
        assert!(info.registered);
        assert_eq!(info.name, "株式会社東急ストア");
        assert_eq!(info.registration_date, "令和5年10月1日");
        assert_eq!(info.address, "東京都目黒区上目黒１丁目２１番１２号");
        assert_eq!(info.last_updated, "令和3年11月19日");
    }

    // --- Simple API tests ---

    #[tokio::test]
    async fn test_lookup_registered() {
        let info = lookup("T8013201004026").await.unwrap();
        assert!(info.registered);
        assert_eq!(info.name, "株式会社東急ストア");
        assert!(!info.registration_date.is_empty());
        assert!(!info.address.is_empty());
        assert!(!info.last_updated.is_empty());
    }

    #[tokio::test]
    async fn test_lookup_unregistered() {
        let info = lookup("T0000000000000").await.unwrap();
        assert!(!info.registered);
    }

    // --- Client reuse tests ---

    #[tokio::test]
    async fn test_client_reuse() {
        let client = InvoiceLookupClient::new();
        // First call (cold: TLS handshake)
        let info1 = client.lookup("T8013201004026").await.unwrap();
        assert!(info1.registered);
        // Second call (warm: reuses connection)
        let info2 = client.lookup("T8013201004026").await.unwrap();
        assert_eq!(info1.name, info2.name);
    }

    // --- Batch tests ---

    #[tokio::test]
    async fn test_batch_lookup() {
        let client = InvoiceLookupClient::new();
        let results = client
            .lookup_batch(&["T8013201004026", "T0000000000000"])
            .await;
        assert_eq!(results.len(), 2);
        assert!(results[0].as_ref().unwrap().registered);
        assert!(!results[1].as_ref().unwrap().registered);
    }
}
