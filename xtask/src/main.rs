//! Repository tasks: `cargo xtask gen-builtins <pg_type.dat>` and
//! `cargo xtask fixture <postgres-url> <out-file>`. See `docs/spec.md`.

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
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
