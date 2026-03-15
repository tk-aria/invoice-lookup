use super::entity::InvoiceRegistration;

/// インボイス登録情報を取得するポート（依存性逆転の原則）
#[async_trait::async_trait]
pub trait InvoiceRepository: Send + Sync {
    async fn find_by_t_number(&self, t_number: &str) -> Result<InvoiceRegistration, String>;
    async fn find_batch(&self, t_numbers: &[String]) -> Vec<Result<InvoiceRegistration, String>>;
}
