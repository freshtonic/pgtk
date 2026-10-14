//! SQL for PostgreSQL.

/// Returns the byte offset of the first `;` in `sql`, if there is one.
pub fn find_statement_end(sql: &str) -> Option<usize> {
    memchr::memchr(b';', sql.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_statement_end() {
        assert_eq!(find_statement_end("SELECT 1; SELECT 2"), Some(8));
        assert_eq!(find_statement_end("SELECT 1"), None);
    }
}
