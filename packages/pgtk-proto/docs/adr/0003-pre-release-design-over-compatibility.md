# Prefer design quality over pre-release compatibility

Until CipherStash Proxy ships rewritten in terms of `pgtk-proto`, prefer the best `pgtk-proto` design over backward compatibility. The crate has no external consumers yet, so obsolete interfaces may be changed or removed without deprecation periods or compatibility adapters; revisit this decision once the rewritten Proxy ships.
