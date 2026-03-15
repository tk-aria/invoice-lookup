use std::sync::Arc;

use crate::domain::entity::InvoiceRegistration;
use crate::domain::repository::InvoiceRepository;

/// 単一T番号の検索ユースケース
pub struct LookupInvoiceUseCase {
    repo: Arc<dyn InvoiceRepository>,
}

impl LookupInvoiceUseCase {
    pub fn new(repo: Arc<dyn InvoiceRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, t_number: &str) -> Result<InvoiceRegistration, String> {
        self.repo.find_by_t_number(t_number).await
    }

    pub async fn execute_batch(
        &self,
        t_numbers: &[String],
    ) -> Vec<Result<InvoiceRegistration, String>> {
        self.repo.find_batch(t_numbers).await
    }
}
