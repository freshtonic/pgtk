# pgtk-proto-fsm

`pgtk-proto-fsm` is the procedural-macro implementation behind
[`pgtk-proto`](https://crates.io/crates/pgtk-proto). It turns one PostgreSQL protocol
grammar into:

- transport-carrying typestate APIs for both protocol roles;
- a runtime FSM used for differential and fuzz testing;
- state-aware wire-message projectors; and
- railroad diagrams embedded in Rustdoc.

Most users should depend on `pgtk-proto`, which re-exports the generated protocol
modules. Depend directly on this crate only when defining or testing another
grammar with the same generator.

## Documentation

- [`pgtk-proto-fsm` Rustdoc](https://docs.rs/pgtk-proto-fsm)
- [`pgtk-proto` Rustdoc](https://docs.rs/pgtk-proto)
- [Repository](https://github.com/freshtonic/pgtk)

## Licence

Licensed under the [MIT License](https://github.com/freshtonic/pgtk/blob/main/packages/pgtk-proto/pgtk-proto-fsm/LICENSE).
