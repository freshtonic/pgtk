//! Repository tasks: `cargo xtask gen-builtins <pg_type.dat>` and
//! `cargo xtask fixture <postgres-url> [--out <file>]`. See `docs/spec.md`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The PostgreSQL release that `pg_type.dat` comes from. Change it when you
/// generate the file from a different release.
const PG_VERSION: &str = "17.9";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("gen-builtins") => {
            if let Err(message) = gen_builtins(&args[1..]) {
                eprintln!("xtask: {message}");
                std::process::exit(1);
            }
        }
        Some("fixture") => {
            if let Err(message) = fixture(&args[1..]) {
                eprintln!("xtask: {message}");
                std::process::exit(1);
            }
        }
        Some(other) => {
            eprintln!("xtask: `{other}` is not implemented yet; see docs/spec.md");
            std::process::exit(2);
        }
        None => {
            eprintln!("usage: cargo xtask <gen-builtins|fixture> ...");
            std::process::exit(2);
        }
    }
}

// --------------------------------------------------------------------------
// gen-builtins
// --------------------------------------------------------------------------

/// Writes `src/builtin/generated.rs` from a `pg_type.dat` file.
fn gen_builtins(args: &[String]) -> Result<(), String> {
    let mut dat: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--out" => {
                let path = rest
                    .next()
                    .ok_or_else(|| "`--out` needs a path".to_string())?;
                out = Some(PathBuf::from(path));
            }
            other if other.starts_with("--") => {
                return Err(format!("unknown option `{other}`"));
            }
            other if dat.is_none() => dat = Some(PathBuf::from(other)),
            other => return Err(format!("unexpected argument `{other}`")),
        }
    }
    let dat = dat.ok_or_else(|| {
        "usage: cargo xtask gen-builtins <pg_type.dat> [--out <file>]".to_string()
    })?;
    let out = out.unwrap_or_else(default_out);

    let text = std::fs::read_to_string(&dat)
        .map_err(|error| format!("cannot read {}: {error}", dat.display()))?;
    let entries = parse_dat(&text)?;
    let builtins = builtins(&entries)?;
    let code = render(&builtins);

    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    std::fs::write(&out, code)
        .map_err(|error| format!("cannot write {}: {error}", out.display()))?;
    format_file(&out);

    println!(
        "xtask: {} entries plus {} arrays = {} variants -> {}",
        entries.len(),
        builtins.len() - entries.len(),
        builtins.len(),
        out.display()
    );
    Ok(())
}

/// The default output path, below the workspace root.
fn default_out() -> PathBuf {
    workspace_root()
        .join("src")
        .join("builtin")
        .join("generated.rs")
}

/// The root of the workspace, from the manifest directory of this crate.
///
/// The path does not depend on the working directory, so a task writes the
/// same file from anywhere.
fn workspace_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().unwrap_or(manifest).to_path_buf()
}

/// Runs `rustfmt` on the written file so that the output is formatted.
fn format_file(path: &Path) {
    match Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .arg(path)
        .status()
    {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!(
            "xtask: rustfmt failed ({status}); format {} by hand",
            path.display()
        ),
        Err(error) => eprintln!(
            "xtask: cannot run rustfmt ({error}); format {} by hand",
            path.display()
        ),
    }
}

// --------------------------------------------------------------------------
// pg_type.dat
// --------------------------------------------------------------------------

/// One `{ ... }` entry of `pg_type.dat`. Only the keys this task uses are
/// kept. Every other key is read and dropped.
#[derive(Debug, PartialEq, Eq)]
struct Entry {
    /// The `oid` key.
    oid: u32,
    /// The `typname` key.
    typname: String,
    /// The `array_type_oid` key, when the entry has one.
    array_type_oid: Option<u32>,
}

/// One variant of the generated `Builtin` enum.
#[derive(Debug, PartialEq, Eq)]
struct Builtin {
    /// The fixed OID.
    oid: u32,
    /// The `typname`.
    name: String,
    /// The Rust variant name.
    variant: String,
}

/// Reads every entry of a `pg_type.dat` file.
///
/// The format is a Perl list of hashes. Comments start with `#`. Values are
/// single-quoted and can contain `\'`. An entry can span lines.
fn parse_dat(text: &str) -> Result<Vec<Entry>, String> {
    let mut scanner = Scanner::new(text);
    let mut entries = Vec::new();
    loop {
        scanner.skip_blanks();
        match scanner.peek() {
            None => break,
            // `{` opens an entry. `[`, `]` and `,` sit between entries.
            Some(b'{') => {
                scanner.bump();
                entries.push(scanner.read_entry()?);
            }
            Some(_) => {
                scanner.bump();
            }
        }
    }
    Ok(entries)
}

/// A character scanner over the text of a `pg_type.dat` file.
struct Scanner<'a> {
    /// The whole file.
    src: &'a [u8],
    /// The read position in `src`.
    pos: usize,
}

impl<'a> Scanner<'a> {
    /// Starts a scanner at the first byte.
    fn new(text: &'a str) -> Self {
        Self {
            src: text.as_bytes(),
            pos: 0,
        }
    }

    /// The byte at the read position.
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    /// Reads one byte and moves the read position.
    fn bump(&mut self) -> Option<u8> {
        let byte = self.peek();
        if byte.is_some() {
            self.pos += 1;
        }
        byte
    }

    /// The line number of the read position, for error messages.
    fn line(&self) -> usize {
        1 + self.src[..self.pos].iter().filter(|b| **b == b'\n').count()
    }

    /// Moves past whitespace and comments.
    fn skip_blanks(&mut self) {
        while let Some(byte) = self.peek() {
            match byte {
                b'#' => {
                    while let Some(comment) = self.bump() {
                        if comment == b'\n' {
                            break;
                        }
                    }
                }
                _ if byte.is_ascii_whitespace() => {
                    self.pos += 1;
                }
                _ => break,
            }
        }
    }

    /// Reads the given text when it is at the read position.
    fn eat(&mut self, want: &[u8]) -> bool {
        if self.src[self.pos..].starts_with(want) {
            self.pos += want.len();
            return true;
        }
        false
    }

    /// Reads one key of a `key => 'value'` pair.
    fn read_key(&mut self) -> Result<String, String> {
        let start = self.pos;
        while let Some(byte) = self.peek() {
            if byte.is_ascii_alphanumeric() || byte == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if start == self.pos {
            return Err(format!("line {}: expected a key", self.line()));
        }
        Ok(String::from_utf8_lossy(&self.src[start..self.pos]).into_owned())
    }

    /// Reads one single-quoted value. `\'` and `\\` are escapes.
    fn read_value(&mut self) -> Result<String, String> {
        let line = self.line();
        if self.bump() != Some(b'\'') {
            return Err(format!("line {line}: expected a quoted value"));
        }
        let mut value = Vec::new();
        loop {
            match self.bump() {
                None => return Err(format!("line {line}: value is not closed")),
                Some(b'\'') => break,
                Some(b'\\') => match self.bump() {
                    None => return Err(format!("line {line}: value is not closed")),
                    Some(escaped @ (b'\'' | b'\\')) => value.push(escaped),
                    Some(other) => {
                        value.push(b'\\');
                        value.push(other);
                    }
                },
                Some(other) => value.push(other),
            }
        }
        Ok(String::from_utf8_lossy(&value).into_owned())
    }

    /// Reads the body of one entry, after its `{`.
    fn read_entry(&mut self) -> Result<Entry, String> {
        let start = self.line();
        let mut oid = None;
        let mut typname = None;
        let mut array_type_oid = None;
        loop {
            self.skip_blanks();
            match self.peek() {
                None => return Err(format!("line {start}: entry is not closed")),
                Some(b'}') => {
                    self.bump();
                    break;
                }
                Some(b',') => {
                    self.bump();
                    continue;
                }
                Some(_) => {}
            }
            let key = self.read_key()?;
            self.skip_blanks();
            if !self.eat(b"=>") {
                return Err(format!("line {}: expected `=>` after `{key}`", self.line()));
            }
            self.skip_blanks();
            let line = self.line();
            let value = self.read_value()?;
            match key.as_str() {
                "oid" => oid = Some(number(&value, line)?),
                "array_type_oid" => array_type_oid = Some(number(&value, line)?),
                "typname" => typname = Some(value),
                _ => {}
            }
        }
        Ok(Entry {
            oid: oid.ok_or_else(|| format!("line {start}: entry has no `oid`"))?,
            typname: typname.ok_or_else(|| format!("line {start}: entry has no `typname`"))?,
            array_type_oid,
        })
    }
}

/// Reads one OID value.
fn number(value: &str, line: usize) -> Result<u32, String> {
    value
        .parse()
        .map_err(|_| format!("line {line}: `{value}` is not an OID"))
}

// --------------------------------------------------------------------------
// Variants
// --------------------------------------------------------------------------

/// Makes the variants of the `Builtin` enum, ordered by OID.
///
/// Each entry makes one variant. An `array_type_oid` makes a second variant
/// for the array of that entry.
fn builtins(entries: &[Entry]) -> Result<Vec<Builtin>, String> {
    let mut builtins = Vec::with_capacity(entries.len() * 2);
    for entry in entries {
        builtins.push(Builtin {
            oid: entry.oid,
            name: entry.typname.clone(),
            variant: variant_name(&entry.typname),
        });
        if let Some(oid) = entry.array_type_oid {
            let name = format!("_{}", entry.typname);
            builtins.push(Builtin {
                oid,
                variant: variant_name(&name),
                name,
            });
        }
    }
    builtins.sort_by_key(|builtin| builtin.oid);
    for pair in builtins.windows(2) {
        if pair[0].oid == pair[1].oid {
            return Err(format!(
                "OID {} is both `{}` and `{}`",
                pair[0].oid, pair[0].name, pair[1].name
            ));
        }
    }
    let mut variants: Vec<&str> = builtins.iter().map(|b| b.variant.as_str()).collect();
    variants.sort_unstable();
    for pair in variants.windows(2) {
        if pair[0] == pair[1] {
            return Err(format!("two types make the variant name `{}`", pair[0]));
        }
    }
    Ok(builtins)
}

/// Makes the Rust variant name of one `typname`.
///
/// The name is split on `_` and each part gets a capital first letter. A
/// leading `_` marks an array: the name loses it and gains `Array`.
fn variant_name(typname: &str) -> String {
    let (base, array) = match typname.strip_prefix('_') {
        Some(base) => (base, true),
        None => (typname, false),
    };
    let mut variant = String::with_capacity(typname.len() + 5);
    for part in base.split('_') {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            variant.extend(first.to_uppercase());
            variant.push_str(chars.as_str());
        }
    }
    if array {
        variant.push_str("Array");
    }
    variant
}

// --------------------------------------------------------------------------
// Output
// --------------------------------------------------------------------------

/// Writes the text of `src/builtin/generated.rs`.
///
/// The code names only `Oid`. It is included from `src/builtin.rs`, so it
/// uses nothing else from the crate.
fn render(builtins: &[Builtin]) -> String {
    let mut code = String::with_capacity(builtins.len() * 200);
    code.push_str(&format!(
        "// GENERATED by `cargo xtask gen-builtins` from pg_type.dat \
         (PostgreSQL {PG_VERSION}). Do not edit.\n\n"
    ));

    code.push_str("/// Every Type with a fixed OID that a SQL expression can name.\n");
    code.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]\n");
    code.push_str("#[repr(u16)]\n");
    code.push_str("pub enum Builtin {\n");
    for (index, builtin) in builtins.iter().enumerate() {
        code.push_str(&format!(
            "    /// `{}`, OID {}.\n",
            builtin.name, builtin.oid
        ));
        if index == 0 {
            code.push_str(&format!("    {} = 0,\n", builtin.variant));
        } else {
            code.push_str(&format!("    {},\n", builtin.variant));
        }
    }
    code.push_str("}\n\n");

    code.push_str("impl Builtin {\n");
    code.push_str("    /// All variants, in discriminant order.\n");
    code.push_str("    pub const ALL: &'static [Builtin] = &[\n");
    for builtin in builtins {
        code.push_str(&format!("        Builtin::{},\n", builtin.variant));
    }
    code.push_str("    ];\n\n");

    code.push_str("    /// The fixed OID.\n");
    code.push_str("    pub const fn oid(self) -> Oid {\n");
    code.push_str("        match self {\n");
    for builtin in builtins {
        code.push_str(&format!(
            "            Builtin::{} => Oid({}),\n",
            builtin.variant, builtin.oid
        ));
    }
    code.push_str("        }\n    }\n\n");

    code.push_str("    /// The `typname`.\n");
    code.push_str("    pub const fn name(self) -> &'static str {\n");
    code.push_str("        match self {\n");
    for builtin in builtins {
        code.push_str(&format!(
            "            Builtin::{} => \"{}\",\n",
            builtin.variant, builtin.name
        ));
    }
    code.push_str("        }\n    }\n");
    code.push_str("}\n");
    code
}

// --------------------------------------------------------------------------
// fixture
// --------------------------------------------------------------------------

/// The compression level of the fixture file.
const ZSTD_LEVEL: i32 = 19;

/// Loads a catalog from a live server and writes it as a fixture file.
///
/// The task connects with the `postgres` crate, runs the loader, and stops
/// when the loader reports a diagnostic. A stock database gives none. The
/// catalog then goes to JSON, and the JSON goes through zstd.
fn fixture(args: &[String]) -> Result<(), String> {
    let (url, out) = fixture_args(args)?;

    let mut client = postgres::Client::connect(&url, postgres::NoTls)
        .map_err(|error| format!("cannot connect to {url}: {error}"))?;
    let loaded =
        pg_catalog::load(&mut client).map_err(|error| format!("the load failed: {error}"))?;
    drop(client);

    if !loaded.diagnostics.is_empty() {
        for diagnostic in &loaded.diagnostics {
            eprintln!("xtask: diagnostic: {diagnostic}");
        }
        return Err(format!(
            "the load reported {} diagnostics; the database is not stock, or the model is wrong",
            loaded.diagnostics.len()
        ));
    }

    let version = loaded.catalog.metadata().server_version;
    let json = serde_json::to_vec(&loaded.catalog)
        .map_err(|error| format!("cannot serialize the catalog: {error}"))?;
    let compressed = zstd::encode_all(json.as_slice(), ZSTD_LEVEL)
        .map_err(|error| format!("cannot compress the catalog: {error}"))?;

    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    std::fs::write(&out, &compressed)
        .map_err(|error| format!("cannot write {}: {error}", out.display()))?;

    println!(
        "xtask: PostgreSQL {version}: {} schemas, {} types, {} relations, {} functions, {} operators, {} casts",
        loaded.catalog.schemas().len(),
        loaded.catalog.types().len(),
        loaded.catalog.relations().len(),
        loaded.catalog.functions().len(),
        loaded.catalog.operators().len(),
        loaded.catalog.casts().len(),
    );
    println!(
        "xtask: {} bytes of JSON, {} bytes after zstd level {} -> {}",
        json.len(),
        compressed.len(),
        ZSTD_LEVEL,
        out.display()
    );
    Ok(())
}

/// Reads the connection string and the output path from the command line.
fn fixture_args(args: &[String]) -> Result<(String, PathBuf), String> {
    let mut url: Option<String> = None;
    let mut out: Option<PathBuf> = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--out" => {
                let path = rest
                    .next()
                    .ok_or_else(|| "`--out` needs a path".to_string())?;
                out = Some(PathBuf::from(path));
            }
            other if other.starts_with("--") => {
                return Err(format!("unknown option `{other}`"));
            }
            other if url.is_none() => url = Some(other.to_owned()),
            other => return Err(format!("unexpected argument `{other}`")),
        }
    }
    let url =
        url.ok_or_else(|| "usage: cargo xtask fixture <postgres-url> [--out <file>]".to_string())?;
    Ok((url, out.unwrap_or_else(default_fixture_out)))
}

/// The default fixture path, below the workspace root.
fn default_fixture_out() -> PathBuf {
    workspace_root().join("fixtures").join("pg17.json.zst")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_names_follow_the_rules() {
        assert_eq!(variant_name("int4"), "Int4");
        assert_eq!(variant_name("_int4"), "Int4Array");
        assert_eq!(variant_name("timestamptz"), "Timestamptz");
        assert_eq!(variant_name("pg_lsn"), "PgLsn");
        assert_eq!(variant_name("int2vector"), "Int2vector");
        assert_eq!(variant_name("_record"), "RecordArray");
        assert_eq!(variant_name("char"), "Char");
        assert_eq!(variant_name("bpchar"), "Bpchar");
    }

    #[test]
    fn parses_entries_over_lines_with_comments_and_escapes() {
        let text = "[\n\
            # a comment with a { brace and a ' quote\n\
            { oid => '16', array_type_oid => '1000',\n\
            \x20 descr => 'boolean, format \\'t\\'/\\'f\\'',\n\
            \x20 typname => 'bool', typlen => '1' },\n\
            { oid => '2249', typname => 'record', typlen => '-1' },\n\
            ]\n";
        let entries = parse_dat(text).expect("parses");
        assert_eq!(
            entries,
            vec![
                Entry {
                    oid: 16,
                    typname: "bool".to_string(),
                    array_type_oid: Some(1000),
                },
                Entry {
                    oid: 2249,
                    typname: "record".to_string(),
                    array_type_oid: None,
                },
            ]
        );
    }

    #[test]
    fn an_array_type_oid_makes_a_second_variant() {
        let entries = vec![Entry {
            oid: 23,
            typname: "int4".to_string(),
            array_type_oid: Some(1007),
        }];
        let builtins = builtins(&entries).expect("builds");
        assert_eq!(builtins[0].variant, "Int4");
        assert_eq!(builtins[1].oid, 1007);
        assert_eq!(builtins[1].name, "_int4");
        assert_eq!(builtins[1].variant, "Int4Array");
    }

    #[test]
    fn a_repeated_oid_is_an_error() {
        let entries = vec![
            Entry {
                oid: 23,
                typname: "int4".to_string(),
                array_type_oid: None,
            },
            Entry {
                oid: 23,
                typname: "other".to_string(),
                array_type_oid: None,
            },
        ];
        assert!(builtins(&entries).is_err());
    }
}
