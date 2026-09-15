//! Helpers that read one text cell of one row.
//!
//! Every cell arrives as text, or as `None` for a SQL NULL. A helper gives
//! back a value, or a [`LoadError::Parse`] that names the query, the row,
//! and the column.
//!
//! A failure here means the query itself is wrong, so the load stops. A
//! value that is odd for one entity only, such as an unknown letter in
//! `prokind`, is not read here. `loader/build.rs` reads those and makes a
//! [`crate::Diagnostic`].

use crate::error::LoadError;
use crate::loader::queries::QueryId;
use crate::name::Name;
use crate::oid::Oid;

/// The text that an error reports for a cell that holds a SQL NULL.
const NULL_TEXT: &str = "NULL";

/// The text that an error reports for a column that the row does not hold.
const MISSING_TEXT: &str = "<missing column>";

/// The cells of one row, with the position that an error reports.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Cells<'a> {
    /// The query that produced the row.
    query: QueryId,
    /// The index of the row, which starts at 0.
    index: usize,
    /// The cells of the row, in the column order of the query.
    cells: &'a [Option<String>],
}

impl<'a> Cells<'a> {
    /// Hold one row of one query.
    pub(crate) fn new(query: QueryId, index: usize, cells: &'a [Option<String>]) -> Self {
        Cells {
            query,
            index,
            cells,
        }
    }

    /// Make the error for one cell.
    fn fail(&self, column: usize, value: &str) -> LoadError {
        LoadError::Parse {
            query: self.query,
            row: self.index,
            column,
            value: value.to_owned(),
        }
    }

    /// The text of a cell that must not hold a SQL NULL.
    pub(crate) fn text(&self, column: usize) -> Result<&'a str, LoadError> {
        match self.cells.get(column) {
            Some(Some(text)) => Ok(text.as_str()),
            Some(None) => Err(self.fail(column, NULL_TEXT)),
            None => Err(self.fail(column, MISSING_TEXT)),
        }
    }

    /// The text of a cell that may hold a SQL NULL.
    pub(crate) fn opt_text(&self, column: usize) -> Result<Option<&'a str>, LoadError> {
        match self.cells.get(column) {
            Some(Some(text)) => Ok(Some(text.as_str())),
            Some(None) => Ok(None),
            None => Err(self.fail(column, MISSING_TEXT)),
        }
    }

    /// An OID.
    pub(crate) fn oid(&self, column: usize) -> Result<Oid, LoadError> {
        let text = self.text(column)?;
        text.parse::<u32>()
            .map(Oid)
            .map_err(|_| self.fail(column, text))
    }

    /// An OID that may be a SQL NULL.
    pub(crate) fn opt_oid(&self, column: usize) -> Result<Option<Oid>, LoadError> {
        match self.opt_text(column)? {
            None => Ok(None),
            Some(text) => text
                .parse::<u32>()
                .map(|value| Some(Oid(value)))
                .map_err(|_| self.fail(column, text)),
        }
    }

    /// A boolean, which the query spells `t` or `f`.
    pub(crate) fn bool(&self, column: usize) -> Result<bool, LoadError> {
        match self.text(column)? {
            "t" => Ok(true),
            "f" => Ok(false),
            other => Err(self.fail(column, other)),
        }
    }

    /// A signed 32-bit number, such as a type modifier.
    pub(crate) fn i32(&self, column: usize) -> Result<i32, LoadError> {
        let text = self.text(column)?;
        text.parse::<i32>().map_err(|_| self.fail(column, text))
    }

    /// A signed 16-bit number, such as an attribute position.
    pub(crate) fn i16(&self, column: usize) -> Result<i16, LoadError> {
        let text = self.text(column)?;
        text.parse::<i16>().map_err(|_| self.fail(column, text))
    }

    /// An unsigned 16-bit number, such as a count of arguments.
    pub(crate) fn u16(&self, column: usize) -> Result<u16, LoadError> {
        let text = self.text(column)?;
        text.parse::<u16>().map_err(|_| self.fail(column, text))
    }

    /// An unsigned 32-bit number, such as a count of constraints.
    pub(crate) fn u32(&self, column: usize) -> Result<u32, LoadError> {
        let text = self.text(column)?;
        text.parse::<u32>().map_err(|_| self.fail(column, text))
    }

    /// A single character, such as `typtype`.
    ///
    /// The cell must hold exactly one character. The meaning of the letter
    /// belongs to `loader/build.rs`.
    pub(crate) fn char(&self, column: usize) -> Result<char, LoadError> {
        let text = self.text(column)?;
        one_char(text).ok_or_else(|| self.fail(column, text))
    }

    /// A single character that may be a SQL NULL, such as `relkind`.
    pub(crate) fn opt_char(&self, column: usize) -> Result<Option<char>, LoadError> {
        match self.opt_text(column)? {
            None => Ok(None),
            Some(text) => one_char(text)
                .map(Some)
                .ok_or_else(|| self.fail(column, text)),
        }
    }

    /// A name, such as `typname`.
    pub(crate) fn name(&self, column: usize) -> Result<Name, LoadError> {
        Ok(Name::new(self.text(column)?))
    }

    /// A name that may be absent.
    ///
    /// A SQL NULL and the empty string both mean that there is no name.
    pub(crate) fn opt_name(&self, column: usize) -> Result<Option<Name>, LoadError> {
        match self.opt_text(column)? {
            None | Some("") => Ok(None),
            Some(text) => Ok(Some(Name::new(text))),
        }
    }
}

/// The one character of a string, when the string holds exactly one.
fn one_char(text: &str) -> Option<char> {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::Cells;
    use crate::error::LoadError;
    use crate::loader::queries::QueryId;
    use crate::name::Name;
    use crate::oid::Oid;

    fn cells(values: &[Option<&str>]) -> Vec<Option<String>> {
        values
            .iter()
            .map(|value| value.map(|text| text.to_owned()))
            .collect()
    }

    #[test]
    fn it_reads_every_kind_of_cell() {
        let row = cells(&[
            Some("23"),
            Some("t"),
            Some("f"),
            Some("-1"),
            Some("2"),
            Some("7"),
            Some("b"),
            Some("int4"),
        ]);
        let row = Cells::new(QueryId::Types, 0, &row);
        assert_eq!(row.oid(0).unwrap(), Oid(23));
        assert!(row.bool(1).unwrap());
        assert!(!row.bool(2).unwrap());
        assert_eq!(row.i32(3).unwrap(), -1);
        assert_eq!(row.i16(4).unwrap(), 2);
        assert_eq!(row.u16(5).unwrap(), 7);
        assert_eq!(row.u32(5).unwrap(), 7);
        assert_eq!(row.char(6).unwrap(), 'b');
        assert_eq!(row.name(7).unwrap(), Name::new("int4"));
    }

    #[test]
    fn a_null_is_none_for_an_optional_cell() {
        let row = cells(&[None, None, None]);
        let row = Cells::new(QueryId::Types, 3, &row);
        assert_eq!(row.opt_text(0).unwrap(), None);
        assert_eq!(row.opt_oid(1).unwrap(), None);
        assert_eq!(row.opt_char(2).unwrap(), None);
    }

    #[test]
    fn an_empty_name_is_no_name() {
        let row = cells(&[Some(""), Some("x"), None]);
        let row = Cells::new(QueryId::FunctionArguments, 0, &row);
        assert_eq!(row.opt_name(0).unwrap(), None);
        assert_eq!(row.opt_name(1).unwrap(), Some(Name::new("x")));
        assert_eq!(row.opt_name(2).unwrap(), None);
    }

    #[test]
    fn a_bad_value_names_the_query_the_row_and_the_column() {
        let row = cells(&[Some("not an oid")]);
        let row = Cells::new(QueryId::Casts, 5, &row);
        match row.oid(0) {
            Err(LoadError::Parse {
                query,
                row,
                column,
                value,
            }) => {
                assert_eq!(query, QueryId::Casts);
                assert_eq!(row, 5);
                assert_eq!(column, 0);
                assert_eq!(value, "not an oid");
            }
            other => panic!("expected a parse error, got {other:?}"),
        }
    }

    #[test]
    fn a_null_in_a_required_cell_is_a_parse_error() {
        let row = cells(&[None]);
        let row = Cells::new(QueryId::Schemas, 0, &row);
        match row.text(0) {
            Err(LoadError::Parse { value, .. }) => assert_eq!(value, "NULL"),
            other => panic!("expected a parse error, got {other:?}"),
        }
    }

    #[test]
    fn a_column_that_the_row_does_not_hold_is_a_parse_error() {
        let row = cells(&[Some("1")]);
        let row = Cells::new(QueryId::Schemas, 0, &row);
        match row.text(4) {
            Err(LoadError::Parse { value, column, .. }) => {
                assert_eq!(value, "<missing column>");
                assert_eq!(column, 4);
            }
            other => panic!("expected a parse error, got {other:?}"),
        }
    }

    #[test]
    fn a_boolean_takes_only_t_and_f() {
        let row = cells(&[Some("true")]);
        let row = Cells::new(QueryId::Types, 0, &row);
        assert!(row.bool(0).is_err());
    }

    #[test]
    fn a_character_cell_takes_only_one_character() {
        let row = cells(&[Some("bb"), Some("")]);
        let row = Cells::new(QueryId::Types, 0, &row);
        assert!(row.char(0).is_err());
        assert!(row.char(1).is_err());
    }
}
