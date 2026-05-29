# JSON envelope compatibility policy

Every `compactp --format json` payload is wrapped in a versioned envelope:

```json
{
  "tool_version":     "0.1.0-beta.1",
  "schema_version":   1,
  "language_version": "0.23.0",
  "input":            "path/to/file.compact",
  "data":             { ... }
}
```

- **`tool_version`** is the `compactp` crate version. It changes every release.
- **`language_version`** is the Compact language version the parser targets
  (confirmed against the installed compiler via `compact compile
  --language-version`). It changes when the validated language surface moves.
- **`schema_version`** is the integer version of the JSON *shape* itself. It is
  the stable contract for machine consumers.

## When `schema_version` bumps

`schema_version` is an integer that increments by 1 on any **backward-incompatible**
change to the JSON shape:

- A field is **removed**.
- A field is **renamed**.
- A field's **type changes** (e.g. string → object).
- The **meaning** of an existing field changes in a way that would break a
  consumer that read the old meaning.

## When `schema_version` does NOT bump

Purely **additive** changes do not bump `schema_version`:

- A **new optional field** is added to the envelope or to a `data` shape.
- A new value appears in an open-ended set (e.g. a new diagnostic `code`), where
  consumers were already expected to tolerate unknown members.

Consumers MUST ignore unknown fields so that additive changes are non-breaking.

## Relationship to the `compactp` version

- A `schema_version` bump is a breaking change to the JSON contract and therefore
  rides a **breaking** `compactp` version bump (a minor bump while in `0.x`; a
  major bump at `1.0+`).
- `tool_version` may advance without a `schema_version` bump (most releases).
- `schema_version` never decreases.

## Current state

| compactp        | schema_version |
| --------------- | -------------- |
| `0.1.0-beta.1`  | `1`            |
