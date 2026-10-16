//! The executor for the `tokio-postgres` crate.

use crate::executor::AsyncExecutor;
use crate::loader::Rows;

impl AsyncExecutor for tokio_postgres::Client {
    type Error = tokio_postgres::Error;

    async fn query(&mut self, sql: &str) -> Result<Rows, Self::Error> {
        let rows = tokio_postgres::Client::query(self, sql, &[]).await?;
        Ok(rows
            .iter()
            .map(|row| {
                (0..row.len())
                    .map(|i| row.get::<_, Option<String>>(i))
                    .collect()
            })
            .collect())
    }
}
