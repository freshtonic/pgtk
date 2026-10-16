//! Look one name up in the stock PostgreSQL 17 catalog and print it.
//!
//! This is the first consumer of the public API. It takes one name, which
//! can be qualified or unqualified, and reports every entity of that name:
//! a type, a relation, every function overload, and every operator
//! overload. An unqualified name goes through the search path that the
//! loader recorded.
//!
//! ```sh
//! cargo run --features fixture --example inspect -- int4
//! cargo run --features fixture --example inspect -- information_schema.tables
//! ```

use pgtk_catalog::{
    Argument, Catalog, Function, Handle, Identity, Operator, QualifiedName, Relation, SearchPath,
    Type, TypeKind,
};

fn main() {
    let Some(input) = std::env::args().nth(1) else {
        eprintln!("usage: inspect <name> or inspect <schema>.<name>");
        std::process::exit(2);
    };

    let catalog = pgtk_catalog::fixture::pg17();
    let path = SearchPath::from_metadata(&catalog);
    let target = Target::parse(&input);

    println!("{}", target.headline(&catalog, &path));

    let mut found = false;
    if let Some(handle) = target.ty(&catalog, &path) {
        found = true;
        print_type(&catalog, handle);
    }
    if let Some(handle) = target.relation(&catalog, &path) {
        found = true;
        print_relation(&catalog, handle);
    }
    let functions = target.functions(&catalog, &path);
    if !functions.is_empty() {
        found = true;
        print_functions(&catalog, &functions);
    }
    let operators = target.operators(&catalog, &path);
    if !operators.is_empty() {
        found = true;
        print_operators(&catalog, &operators);
    }

    if !found {
        println!("  nothing of that name");
        std::process::exit(1);
    }
}

/// The name to look up, either qualified or not.
enum Target {
    /// A name with a schema, which the lookup uses as it is.
    Qualified(QualifiedName),
    /// A bare name, which the lookup reads along the search path.
    Bare(String),
}

impl Target {
    /// Read the name from the command line.
    ///
    /// One dot splits the schema from the entity. An operator name holds no
    /// dot, so the split is safe for `+` and its kin.
    fn parse(input: &str) -> Self {
        match input.split_once('.') {
            Some((schema, name)) => Target::Qualified(QualifiedName::new(schema, name)),
            None => Target::Bare(input.to_owned()),
        }
    }

    /// The line that names what the lookup did.
    fn headline(&self, catalog: &Catalog, path: &SearchPath) -> String {
        match self {
            Target::Qualified(name) => format!("{name}, in that schema"),
            Target::Bare(name) => {
                let schemas: Vec<&str> = path
                    .schemas()
                    .iter()
                    .map(|handle| catalog[*handle].name.as_str())
                    .collect();
                format!("{name}, on search path {}", schemas.join(", "))
            }
        }
    }

    /// The type of this name.
    fn ty(&self, catalog: &Catalog, path: &SearchPath) -> Option<Handle<Type>> {
        match self {
            Target::Qualified(name) => catalog.type_named(name),
            Target::Bare(name) => catalog.type_visible(path, name),
        }
    }

    /// The relation of this name.
    fn relation(&self, catalog: &Catalog, path: &SearchPath) -> Option<Handle<Relation>> {
        match self {
            Target::Qualified(name) => catalog.relation_named(name),
            Target::Bare(name) => catalog.relation_visible(path, name),
        }
    }

    /// Every function of this name.
    fn functions(&self, catalog: &Catalog, path: &SearchPath) -> Vec<Handle<Function>> {
        match self {
            Target::Qualified(name) => catalog.functions_named(name).to_vec(),
            Target::Bare(name) => catalog.functions_visible(path, name),
        }
    }

    /// Every operator of this name.
    fn operators(&self, catalog: &Catalog, path: &SearchPath) -> Vec<Handle<Operator>> {
        match self {
            Target::Qualified(name) => catalog.operators_named(name).to_vec(),
            Target::Bare(name) => catalog.operators_visible(path, name),
        }
    }
}

/// The name of a type, with its schema when that schema is not `pg_catalog`.
fn type_name(catalog: &Catalog, handle: Handle<Type>) -> String {
    let name = catalog.qualified_name_of_type(handle);
    if name.schema.as_str() == "pg_catalog" {
        name.name.to_string()
    } else {
        name.to_string()
    }
}

/// The kind of a type, with the detail that belongs to it.
fn kind_of(catalog: &Catalog, handle: Handle<Type>) -> String {
    match &catalog[handle].kind {
        TypeKind::Base => "base".to_owned(),
        TypeKind::Array { element } => format!("array of {}", type_name(catalog, *element)),
        TypeKind::Composite(detail) => {
            /// How many fields the line names before it counts the rest.
            const SHOWN: usize = 4;
            let mut fields: Vec<String> = detail
                .fields
                .iter()
                .take(SHOWN)
                .map(|field| format!("{} {}", field.name, type_name(catalog, field.ty)))
                .collect();
            if detail.fields.len() > SHOWN {
                fields.push(format!("{} more", detail.fields.len() - SHOWN));
            }
            let relation = match detail.relation {
                Some(relation) => format!(", row type of {}", catalog[relation].name),
                None => String::new(),
            };
            format!("composite ({}){}", fields.join(", "), relation)
        }
        TypeKind::Domain(detail) => format!("domain over {}", type_name(catalog, detail.base)),
        TypeKind::Enum { labels } => {
            let labels: Vec<&str> = labels.iter().map(|label| label.as_str()).collect();
            format!("enum ({})", labels.join(", "))
        }
        TypeKind::Range(detail) => format!("range of {}", type_name(catalog, detail.subtype)),
        TypeKind::Multirange { range } => {
            format!("multirange of {}", type_name(catalog, *range))
        }
        TypeKind::Pseudo(pseudo) => {
            let polymorphic = if pseudo.is_polymorphic() {
                ", polymorphic"
            } else {
                ""
            };
            format!("pseudo-type {pseudo:?}{polymorphic}")
        }
    }
}

/// Print a type, its kind, and its array type.
fn print_type(catalog: &Catalog, handle: Handle<Type>) {
    let ty = &catalog[handle];
    println!("\ntype {} (oid {})", type_name(catalog, handle), ty.oid);
    println!("  kind      {}", kind_of(catalog, handle));
    println!("  category  {:?} ({})", ty.category, ty.category.as_char());
    match ty.array {
        Some(array) => println!("  array     {}", type_name(catalog, array)),
        None => println!("  array     none"),
    }
    if ty.is_preferred {
        println!("  preferred in its category");
    }
    match ty.collation {
        Some(collation) => println!(
            "  collation {}",
            catalog.qualified_name_of_collation(collation)
        ),
        None if ty.is_collatable => println!("  takes a collation"),
        None => {}
    }
    if let Some(builtin) = catalog.as_builtin(handle) {
        println!("  built-in  {builtin:?}");
    }
}

/// Print a relation and every column.
fn print_relation(catalog: &Catalog, handle: Handle<Relation>) {
    let relation = &catalog[handle];
    println!(
        "\nrelation {} (oid {})",
        catalog.qualified_name_of_relation(handle),
        relation.oid
    );
    println!("  kind      {:?}", relation.kind);
    match relation.row_type {
        Some(row_type) => println!("  row type  {}", type_name(catalog, row_type)),
        None => println!("  row type  none"),
    }
    if let Some(text) = &relation.view_definition {
        println!(
            "  view      {}",
            text.split_whitespace().collect::<Vec<_>>().join(" ")
        );
    }
    for constraint in &relation.constraints {
        println!(
            "  constraint {} {:?} {:?}",
            constraint.name, constraint.kind, constraint.columns
        );
    }
    for rule in &relation.rules {
        println!(
            "  rule      {} on {:?}{}{} ({:?})",
            rule.name,
            rule.event,
            if rule.instead { " instead" } else { " also" },
            if rule.unconditional { "" } else { " where ..." },
            rule.fires
        );
    }
    for trigger in &relation.triggers {
        println!(
            "  trigger   {} {:?} ({:?})",
            trigger.name, trigger.timing, trigger.fires
        );
    }
    if let Some(of_type) = relation.of_type {
        println!("  of type   {}", type_name(catalog, of_type));
    }
    if let Some(key) = &relation.primary_key {
        let names: Vec<&str> = key
            .columns
            .iter()
            .filter_map(|position| relation.column_at(*position))
            .map(|column| column.name.as_str())
            .collect();
        let note = if key.deferrable { " (deferrable)" } else { "" };
        println!("  key       ({}){note}", names.join(", "));
    }
    println!("  {} columns", relation.columns.len());
    for column in &relation.columns {
        let mut notes = Vec::new();
        if column.not_null {
            notes.push("not null");
        }
        if column.has_default {
            notes.push("default");
        }
        if column.generated {
            notes.push("generated");
        }
        match column.identity {
            Some(Identity::Always) => notes.push("identity always"),
            Some(Identity::ByDefault) => notes.push("identity by default"),
            None => {}
        }
        let line = format!(
            "    {:>3}  {:<24} {:<24} {}",
            column.position,
            column.name,
            type_name(catalog, column.ty),
            notes.join(" ")
        );
        println!("{}", line.trim_end());
    }
}

/// One argument, as a signature writes it.
fn argument(catalog: &Catalog, argument: &Argument) -> String {
    let mode = match argument.mode {
        pgtk_catalog::ArgMode::In => "",
        pgtk_catalog::ArgMode::Out => "OUT ",
        pgtk_catalog::ArgMode::InOut => "INOUT ",
        pgtk_catalog::ArgMode::Variadic => "VARIADIC ",
        pgtk_catalog::ArgMode::Table => "TABLE ",
    };
    let name = match &argument.name {
        Some(name) => format!("{name} "),
        None => String::new(),
    };
    format!("{mode}{name}{}", type_name(catalog, argument.ty))
}

/// Print every overload, with its argument types and its return type.
fn print_functions(catalog: &Catalog, handles: &[Handle<Function>]) {
    println!("\n{} function overloads", handles.len());
    for handle in handles {
        let function = &catalog[*handle];
        let arguments: Vec<String> = function
            .arguments
            .iter()
            .map(|a| argument(catalog, a))
            .collect();
        let returns = if function.returns_set {
            format!("SETOF {}", type_name(catalog, function.returns))
        } else {
            type_name(catalog, function.returns)
        };
        println!(
            "  {}({}) -> {}",
            catalog.qualified_name_of_function(*handle),
            arguments.join(", "),
            returns
        );
        let mut notes = vec![
            format!("{:?}", function.kind),
            format!("{:?}", function.volatility),
        ];
        if function.strict {
            notes.push("strict".to_owned());
        }
        if let Some(variadic) = function.variadic {
            notes.push(format!("variadic over {}", type_name(catalog, variadic)));
        }
        if function.default_count > 0 {
            let types: Vec<String> = function
                .default_types
                .iter()
                .map(|ty| ty.map_or_else(|| "?".to_owned(), |ty| type_name(catalog, ty)))
                .collect();
            notes.push(format!(
                "{} defaults ({})",
                function.default_count,
                types.join(", ")
            ));
        }
        if let Some(aggregate) = &function.aggregate {
            notes.push(format!(
                "aggregate {:?}, {} direct args",
                aggregate.kind, aggregate.direct_args
            ));
        }
        println!("      {}", notes.join(", "));
    }
}

/// Print every operator overload.
fn print_operators(catalog: &Catalog, handles: &[Handle<Operator>]) {
    println!("\n{} operator overloads", handles.len());
    for handle in handles {
        let operator = &catalog[*handle];
        let left = match operator.left {
            Some(left) => format!("{} ", type_name(catalog, left)),
            None => String::new(),
        };
        println!(
            "  {}{} {} -> {}  [{}]",
            left,
            catalog.qualified_name_of_operator(*handle),
            type_name(catalog, operator.right),
            type_name(catalog, operator.result),
            catalog.qualified_name_of_function(operator.function)
        );
    }
}
