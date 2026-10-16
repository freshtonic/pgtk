//! The executor for the `postgres` crate.

use crate::executor::Executor;
use crate::loader::Rows;

impl Executor for postgres::Client {
    type Error = postgres::Error;

    fn query(&mut self, sql: &str) -> Result<Rows, Self::Error> {
        let rows = postgres::Client::query(self, sql, &[])?;
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
