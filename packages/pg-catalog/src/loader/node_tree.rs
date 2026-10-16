//! A small reader of the text form of a `pg_node_tree`.
//!
//! The server writes an expression tree as text, with `nodeToString`. A node
//! is `{LABEL :field value ...}`. A list is `(item item ...)`. An empty list is
//! `<>`. A backslash comes before each special character of a string, so a
//! brace or a parenthesis that follows a backslash is not structure.
//!
//! The loader reads `pg_proc.proargdefaults` only, and needs only one fact
//! from it: the result type of each default expression. That is what
//! `exprType()` gives in the server. For the node kinds below, the node holds
//! its result type in one field, so the reader does not examine the
//! arguments of the node.

use crate::oid::Oid;

/// The OID of `bool`. A test node always gives this type.
const BOOL: Oid = Oid(16);

/// Where a node kind holds its result type.
enum ResultType {
    /// The field with this name holds the OID.
    Field(&'static str),
    /// The node always gives `bool`.
    Bool,
}

/// The place of the result type of a node kind, by the label of the node.
///
/// PostgreSQL 16 changed some labels, so both forms are here. A label that
/// is not here gives no type.
fn result_type_of(label: &str) -> Option<ResultType> {
    let place = match label {
        "CONST" => ResultType::Field(":consttype"),
        "PARAM" => ResultType::Field(":paramtype"),
        "FUNCEXPR" => ResultType::Field(":funcresulttype"),
        "OPEXPR" | "DISTINCTEXPR" | "NULLIFEXPR" => ResultType::Field(":opresulttype"),
        "RELABELTYPE" | "COERCEVIAIO" | "ARRAYCOERCEEXPR" | "COERCETODOMAIN" | "FIELDSELECT"
        | "CONVERTROWTYPEEXPR" => ResultType::Field(":resulttype"),
        "ARRAY" | "ARRAYEXPR" => ResultType::Field(":array_typeid"),
        "SUBSCRIPTINGREF" => ResultType::Field(":refrestype"),
        "CASE" | "CASEEXPR" => ResultType::Field(":casetype"),
        "COALESCE" | "COALESCEEXPR" => ResultType::Field(":coalescetype"),
        "MINMAX" | "MINMAXEXPR" => ResultType::Field(":minmaxtype"),
        "ROW" | "ROWEXPR" => ResultType::Field(":row_typeid"),
        "SQLVALUEFUNCTION" => ResultType::Field(":type"),
        "BOOLEXPR" | "NULLTEST" | "BOOLEANTEST" | "SCALARARRAYOPEXPR" => ResultType::Bool,
        _ => return None,
    };
    Some(place)
}

/// The result type of each expression in a list of expressions.
///
/// The text is the value of `proargdefaults`. The result has one entry for
/// each expression, in order. An entry is `None` when the reader does not
/// know the node kind. The whole result is `None` when the text is not a
/// list of nodes.
pub(crate) fn expression_types(text: &str) -> Option<Vec<Option<Oid>>> {
    let text = text.trim();
    if text == "<>" {
        return Some(Vec::new());
    }
    let inner = text.strip_prefix('(')?.strip_suffix(')')?;
    top_level_nodes(inner)?
        .into_iter()
        .map(|node| Some(node_type(node)))
        .collect()
}

/// Split the inside of a list into its nodes.
///
/// Each item is the text between the braces of one node. The result is
/// `None` when the braces do not balance, and when an item is not a node.
fn top_level_nodes(list: &str) -> Option<Vec<&str>> {
    let mut nodes = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut chars = list.char_indices();
    while let Some((index, character)) = chars.next() {
        match character {
            '\\' => {
                chars.next();
            }
            '{' => {
                if depth == 0 {
                    start = index + 1;
                }
                depth += 1;
            }
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    nodes.push(&list[start..index]);
                }
            }
            character if depth == 0 && !character.is_whitespace() => return None,
            _ => {}
        }
    }
    (depth == 0).then_some(nodes)
}

/// The result type of one node, from the text between its braces.
fn node_type(node: &str) -> Option<Oid> {
    let tokens = own_tokens(node);
    let label = tokens.first()?;
    match result_type_of(label)? {
        ResultType::Bool => Some(BOOL),
        ResultType::Field(field) => {
            let position = tokens.iter().position(|token| *token == field)?;
            let oid: u32 = tokens.get(position + 1)?.parse().ok()?;
            (oid != 0).then_some(Oid(oid))
        }
    }
}

/// The tokens of a node that are not inside a nested node or a nested list.
///
/// A field of a nested node can have the name of a field of this node. The
/// depth count keeps those apart.
fn own_tokens(node: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut depth = 0usize;
    let mut start: Option<usize> = None;
    let mut chars = node.char_indices();
    while let Some((index, character)) = chars.next() {
        let is_open = matches!(character, '{' | '(');
        let is_close = matches!(character, '}' | ')');
        if character == '\\' {
            chars.next();
            if depth == 0 && start.is_none() {
                start = Some(index);
            }
            continue;
        }
        if depth == 0 && (character.is_whitespace() || is_open) {
            if let Some(from) = start.take() {
                tokens.push(&node[from..index]);
            }
        } else if depth == 0 && start.is_none() && !is_close {
            start = Some(index);
        }
        if is_open {
            depth += 1;
        } else if is_close {
            depth = depth.saturating_sub(1);
        }
    }
    if let Some(from) = start {
        tokens.push(&node[from..]);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::expression_types;
    use crate::oid::Oid;

    /// A `text` constant, as PostgreSQL 17 writes it.
    const TEXT_CONST: &str = "{CONST :consttype 25 :consttypmod -1 :constcollid 100 \
        :constlen -1 :constbyval false :constisnull false :location -1 \
        :constvalue 9 [ 36 0 0 0 87 111 114 108 100 ]}";

    /// An `int4` constant.
    const INT4_CONST: &str = "{CONST :consttype 23 :consttypmod -1 :constcollid 0 \
        :constlen 4 :constbyval true :constisnull false :location -1 \
        :constvalue 4 [ 1 0 0 0 0 0 0 0 ]}";

    #[test]
    fn a_constant_gives_its_consttype() {
        let text = format!("({TEXT_CONST} {INT4_CONST})");
        assert_eq!(
            expression_types(&text),
            Some(vec![Some(Oid(25)), Some(Oid(23))])
        );
    }

    #[test]
    fn an_untyped_literal_gives_the_unknown_type() {
        let text = "({CONST :consttype 705 :consttypmod -1 :constcollid 0 :constlen -2 \
            :constbyval false :constisnull false :location -1 :constvalue 2 [ 120 0 ]})";
        assert_eq!(expression_types(text), Some(vec![Some(Oid(705))]));
    }

    #[test]
    fn an_operator_gives_its_result_type_and_not_that_of_an_argument() {
        // `1 + 1` for a `float8` result, to show that the inner `consttype`
        // does not win.
        let text = format!(
            "({{OPEXPR :opno 591 :opfuncid 218 :opresulttype 701 :opretset false \
             :opcollid 0 :inputcollid 0 :args ({INT4_CONST} {INT4_CONST}) :location -1}})"
        );
        assert_eq!(expression_types(&text), Some(vec![Some(Oid(701))]));
    }

    #[test]
    fn a_field_after_the_arguments_is_found() {
        let text = format!(
            "({{ROWEXPR :args ({INT4_CONST}) :row_typeid 2249 :row_format 2 \
             :colnames <> :location -1}})"
        );
        assert_eq!(expression_types(&text), Some(vec![Some(Oid(2249))]));
    }

    #[test]
    fn an_array_has_one_label_before_postgresql_16_and_another_after() {
        for label in ["ARRAY", "ARRAYEXPR"] {
            let text = format!(
                "({{{label} :array_typeid 1007 :array_collid 0 :element_typeid 23 \
                 :elements ({INT4_CONST}) :multidims false :location -1}})"
            );
            assert_eq!(expression_types(&text), Some(vec![Some(Oid(1007))]));
        }
    }

    #[test]
    fn a_function_call_with_no_argument_gives_its_result_type() {
        let text = "({FUNCEXPR :funcid 1299 :funcresulttype 1184 :funcretset false \
            :funcvariadic false :funcformat 0 :funccollid 0 :inputcollid 0 :args <> \
            :location -1})";
        assert_eq!(expression_types(text), Some(vec![Some(Oid(1184))]));
    }

    #[test]
    fn a_test_node_gives_bool() {
        let text = format!(
            "({{NULLTEST :arg {INT4_CONST} :nulltesttype 0 :argisrow false :location -1}})"
        );
        assert_eq!(expression_types(&text), Some(vec![Some(Oid(16))]));
    }

    #[test]
    fn an_unknown_node_gives_none_and_keeps_its_place() {
        let text = format!("({INT4_CONST} {{NEWNODE :sometype 23}} {TEXT_CONST})");
        assert_eq!(
            expression_types(&text),
            Some(vec![Some(Oid(23)), None, Some(Oid(25))])
        );
    }

    #[test]
    fn an_escaped_brace_is_not_structure() {
        let text =
            format!("({{NEWNODE :name a\\{{b\\}}\\ \\(c\\) :args ({INT4_CONST})}} {INT4_CONST})");
        assert_eq!(expression_types(&text), Some(vec![None, Some(Oid(23))]));
    }

    #[test]
    fn the_empty_list_has_no_entry() {
        assert_eq!(expression_types("<>"), Some(Vec::new()));
    }

    #[test]
    fn text_that_is_not_a_list_of_nodes_gives_none() {
        assert_eq!(expression_types("{CONST :consttype 23}"), None);
        assert_eq!(expression_types("({CONST :consttype 23)"), None);
        assert_eq!(expression_types("(23 24)"), None);
        assert_eq!(expression_types(""), None);
    }
}
