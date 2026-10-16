# Entities reference each other through typed handles into arenas

A Catalog stores each kind of entity in an arena owned by the Catalog, and
entities reference each other with newtype handles (a `TypeId` cannot index
the function arena). We rejected OIDs as the Rust identity because they force
a lookup at every edge and cannot be checked for kind; we rejected `Arc`
graphs because the catalog has cycles (composite type and its relation, array
and element), which make serialization and equality awkward. Handles are
copyable, serialize as integers, and cannot dangle because a Catalog is
immutable after load. Every entity still carries its OID for display.
