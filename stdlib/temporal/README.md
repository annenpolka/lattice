# lattice/std/temporal

`freeze` is hosted as a Wasmtime WIT component (`lattice:stdlib/lowering.freeze`). `flow` remains a host builtin.

| Word | Effect |
|---|---|
| `freeze` | Insert a rate-0 TimeMap segment |
| `flow` | Sequence body order is scene order |

Do not add a `match "freeze"` arm to `lattice-vel`.
