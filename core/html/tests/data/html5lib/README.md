# html5lib tokenizer cases

Vendored, unmodified, from <https://github.com/html5lib/html5lib-tests> (`tokenizer/`), commit
`c777c408b61078ea2eb4acefc2535f54dbc8b28a`, fetched 2026-10-06. MIT licensed — see `LICENSE`.

| file                 | cases | sha256                                                             |
| -------------------- | ----- | ------------------------------------------------------------------ |
| `namedEntities.test` | 4210  | `a7f0e59ff7653820330548776cb3031c18e45f5fd1481a9813d9c7acee89bd6e` |
| `entities.test`      | 80    | `fe17483810a00247579f5f129ca9c007fbab6755ba839523e29aa9f8875f4085` |

`core/html/tests/html5lib_entities.rs` replays every case and asserts the emitted tokens, the parse-error codes and
their locations. There is no skip list (issue #31). The named-reference table was generated from a different source
(`core/html/src/domain/named_references.rs`), so these cases are an independent check of it, not a restatement.
