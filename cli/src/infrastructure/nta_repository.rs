use invoice_lookup::InvoiceLookupClient;

use crate::domain::entity::InvoiceRegistration;
use crate::domain::repository::InvoiceRepository;

/// NTA公表サイトを使ったInvoiceRepositoryの実装
pub struct NtaWebRepository {
    client: InvoiceLookupClient,
}

impl NtaWebRepository {
    pub fn new() -> Self {
        Self {
            client: InvoiceLookupClient::new(),
        }
    }
}

#[async_trait::async_trait]
impl InvoiceRepository for NtaWebRepository {
    async fn find_by_t_number(&self, t_number: &str) -> Result<InvoiceRegistration, String> {
        let info = self
            .client
            .lookup(t_number)
            .await
            .map_err(|e| e.to_string())?;
        Ok(InvoiceRegistration {
            t_number: t_number.to_string(),
            name: info.name,
            registration_date: info.registration_date,
            address: info.address,
            last_updated: info.last_updated,
            registered: info.registered,
        })
    }

    async fn find_batch(&self, t_numbers: &[String]) -> Vec<Result<InvoiceRegistration, String>> {
        let refs: Vec<&str> = t_numbers.iter().map(|s| s.as_str()).collect();
        let results = self.client.lookup_batch(&refs).await;
        results
            .into_iter()
            .zip(t_numbers.iter())
            .map(|(result, tnum)| {
                result
                    .map(|info| InvoiceRegistration {
                        t_number: tnum.clone(),
                        name: info.name,
                        registration_date: info.registration_date,
                        address: info.address,
                        last_updated: info.last_updated,
                        registered: info.registered,
                    })
                    .map_err(|e| e.to_string())
            })
            .collect()
    }
}
