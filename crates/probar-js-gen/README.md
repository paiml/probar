# probar-js-gen

NASA/DO-178B-grade Rust DSL for type-safe JavaScript generation.

## Features

- **Type Safety**: Invalid JS constructs are unrepresentable in the type system
- **Identifier Validation**: Reserved words and invalid characters are rejected at construction
- **Immutability Enforcement**: Generated files include Blake3 hash manifests for verification
- **Deterministic Output**: Same HIR always produces identical JavaScript

## Usage

```rust
use probar_js_gen::prelude::*;

// Build a JavaScript module
let module = JsModuleBuilder::new()
    .comment("Generated - DO NOT EDIT")
    .let_decl("x", Expr::num(42))?
    .const_decl("msg", Expr::str("hello"))?
    .build();

// Generate JavaScript code
let js = generate(&module);

// Write with manifest for immutability verification
write_with_manifest(
    Path::new("./output.js"),
    &js,
    GenerationMetadata {
        tool: "my-tool".to_string(),
        version: "1.0.0".to_string(),
        input_hash: blake3::hash(spec).to_hex().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        regenerate_cmd: "my-tool gen --spec input.spec".to_string(),
    },
)?;

// Later, verify the file wasn't modified
verify(Path::new("./output.js"))?;
```

## Quality Guarantees

- 95%+ test coverage
- 85%+ mutation score
- Property-based testing with proptest
- Fuzz testing with cargo-fuzz
- Zero forbidden patterns in generated output

## License

MIT OR Apache-2.0
