# CLAUDE.md - AI Assistant Guide for anyhow

## Project Overview

**anyhow** is a Rust library providing flexible, trait-object-based error handling for idiomatic error management in Rust applications. It offers `anyhow::Error` and `anyhow::Result<T>` as alternatives to manual error type definitions.

- **Version**: 1.0.100
- **Maintainer**: David Tolnay (dtolnay@gmail.com)
- **Edition**: Rust 2024
- **MSRV**: Rust 1.93
- **License**: MIT OR Apache-2.0

## Repository Structure

```
anyhow/
├── src/                    # Main library source
│   ├── lib.rs             # Library root, public API exports
│   ├── error.rs           # Core Error type implementation
│   ├── ensure.rs          # ensure! macro internals (complex)
│   ├── macros.rs          # anyhow!, bail!, ensure! macro definitions
│   ├── context.rs         # Context trait for Result/Option
│   ├── backtrace.rs       # Backtrace capture and display (std::backtrace)
│   ├── chain.rs           # Error chain iterator
│   ├── kind.rs            # Tagged dispatch for macro type resolution
│   ├── fmt.rs             # Error formatting (Display/Debug)
│   ├── ptr.rs             # Pointer wrapper types (Own, Ref, Mut)
│   ├── wrapper.rs         # Error wrapper types
│   └── nightly.rs         # Nightly-only features (error_generic_member_access)
├── tests/                  # Integration tests
│   ├── test_*.rs          # Various test modules
│   ├── ui/                # Compile-fail tests (trybuild)
│   ├── common/            # Shared test utilities
│   ├── drop/              # Drop detection utilities
│   └── crate/             # Crate integration tests
├── build.rs               # Build script with feature detection
├── Cargo.toml             # Package manifest
└── .github/workflows/ci.yml  # CI configuration
```

## Key Architecture Concepts

### Error Representation
- `Error` is a single-word pointer (not two like `Box<dyn Error>`)
- Uses `Own<ErrorImpl>` for optimized memory layout
- Supports downcasting by value, shared reference, or mutable reference

### Three Public Macros
- **`anyhow!`** - Construct Error from string/format/Display+Debug types
- **`bail!`** - Early return with an Error
- **`ensure!`** - Conditional assertion with smart error messages

### Feature Detection (build.rs)
The build script probes compiler capabilities at build time:
- Probes for `error_generic_member_access` (nightly)
- Emits `cargo:rustc-cfg` flags for conditional compilation

### Conditional Compilation
- `std` feature (default) - Full standard library support
- `std_backtrace` - Always enabled with MSRV 1.93 (std::backtrace stable since 1.65)
- No-std mode - Disable `std` feature, requires global allocator

## Development Commands

### Building
```bash
cargo build                    # Standard build
cargo build --no-default-features  # No-std build
```

### Testing
```bash
cargo test                     # Run all tests
cargo test --no-default-features   # Test no-std mode
```

### Linting
```bash
cargo clippy --tests -- -Dclippy::all -Dclippy::pedantic
```

### Documentation
```bash
cargo doc --open               # Generate and view docs
```

### Memory Safety Checks
```bash
cargo miri test                # Run under Miri (requires nightly)
```

## Testing Conventions

### Test Types
1. **Unit tests** - Inline in source files
2. **Integration tests** - `tests/test_*.rs` files
3. **Compile-fail tests** - `tests/ui/*.rs` with `.stderr` expectations (uses trybuild)

### Running UI Tests
UI tests require nightly Rust and are gated with `rustversion`:
```bash
rustup run nightly cargo test
```

### Test Features
- Drop detection in `tests/drop/` verifies proper cleanup
- `tests/common/` contains shared test utilities
- `tests/crate/` tests minimal dependency builds

## CI Requirements

All changes must pass:
- Tests on Rust nightly, beta, stable
- MSRV check on Rust 1.93.0
- No-std build check
- Clippy with pedantic lints (nightly)
- Miri for undefined behavior detection
- Documentation generation
- Windows build

**Important**: `RUSTFLAGS=-Dwarnings` is set - all warnings are errors.

## Code Conventions

### Error Handling
- All errors must implement `std::error::Error` (or `core::error::Error` in no-std)
- Use `Context` trait for adding context to errors
- Prefer `?` operator for error propagation

### Macro Implementation
- `kind.rs` uses autoref-based dispatch to specialize between:
  - `AdhocKind` - Types with Display+Debug
  - `TraitKind` - Types implementing std::error::Error
  - `BoxedKind` - Boxed trait objects

### Unsafe Code
- Carefully audited and documented
- Primarily in `ptr.rs` for pointer manipulation
- `error.rs` contains layout optimization unsafe code

### Edition 2024 Patterns
- Use `#[unsafe(no_mangle)]` for FFI functions (not `#[no_mangle]`)
- Use `&mut`/`&` prefix in match patterns when mutably binding through references
- Use `io::Error::other()` instead of `io::Error::new(ErrorKind::Other, ...)`

### Documentation Standards
- Comprehensive rustdoc with examples on public APIs
- Examples are tested as doctests
- Internal modules should have `//!` module-level documentation
- Document non-obvious internal functions with `///` doc comments
- Document vtable/trait signature constraints when they affect implementation

## Code Quality Requirements

### Avoiding Hacks and Workarounds

**Do NOT use these patterns:**

1. **`let _ = value;`** - Never use this to silence unused variable warnings
   - Instead: Use the variable meaningfully (e.g., `debug_assert!`) or use `#[allow(unused_variables)]` with documentation explaining why

2. **`_variable` prefix as a silencing hack** - Avoid when the variable could be used meaningfully
   - Acceptable: In function parameters constrained by trait/vtable signatures where usage isn't possible
   - Required: Documentation explaining why the parameter is unused

3. **`#[allow(clippy::...)]`** - Avoid unless absolutely necessary
   - Required: Documentation explaining the constraint that prevents fixing the lint
   - Preferred: Restructure code to satisfy the lint

### Explicit Lifetimes

**Use explicit lifetime parameters instead of `'_` when:**

1. **Standalone functions** where input/output lifetime relationships matter:
   ```rust
   // Good: Explicit relationship between input and output lifetimes
   fn get_ref<'a>(e: Ref<'a, ErrorImpl>) -> Option<&'a Backtrace>

   // Avoid: Anonymous lifetime hides the relationship
   fn get_ref(e: Ref<'_, ErrorImpl>) -> Option<&Backtrace>
   ```

2. **`'_` is acceptable** in method return types clearly tied to `&self`:
   ```rust
   // Acceptable: Lifetime obviously tied to &self
   fn by_ref(&self) -> Ref<'_, T>
   ```

### Vtable and Trait Constraints

When implementing functions for vtables or trait objects:

1. Document why the signature is constrained
2. Use `#[allow(unused_variables)]` with explanation when parameters can't be used
3. Use `debug_assert!` to validate invariants when parameters must exist for signature compatibility

### Code Style Principles

- **Explicit over implicit** - lifetimes, types, error handling
- **No backwards-compatibility shims** - remove dead code completely
- **No re-exports or aliases** just to silence warnings
- **No `#[allow(dead_code)]`** - remove unused code entirely
- **Idiomatic Rust 2024** - adopt new patterns, don't preserve old ones

## Important Files for Understanding the Codebase

| File | Purpose |
|------|---------|
| `src/lib.rs` | Public API, type exports, module structure |
| `src/error.rs` | Core `Error` type, downcasting, conversions |
| `src/macros.rs` | Public macro definitions and docs |
| `src/context.rs` | `Context` trait implementation |
| `build.rs` | Feature detection and conditional compilation |

## Common Modification Patterns

### Adding a New Method to Error
1. Implement in `src/error.rs`
2. Add tests in `tests/test_*.rs`
3. Document with examples in rustdoc

### Modifying Macro Behavior
1. Update macro in `src/macros.rs`
2. Internal parsing logic is in `src/ensure.rs` (for ensure!)
3. Add/update compile-fail tests in `tests/ui/`
4. Run UI tests with nightly to update `.stderr` files

### Supporting a New Rust Feature
1. Add feature probe in `build.rs`
2. Use `#[cfg(feature_name)]` for conditional compilation
3. Test on multiple Rust versions

## Gotchas and Tips

- **UI tests require nightly** - `.stderr` files are version-specific
- **build.rs creates probe directory** - Cleaned up automatically but may leave artifacts
- **Single-word Error size** - Breaking this invariant would be a breaking change
- **Sealed traits** - `Context` trait is sealed, cannot be implemented externally
- **MSRV 1.93** - Don't use features unavailable in Rust 1.93
- **std::backtrace always available** - MSRV 1.93 > 1.65 when it was stabilized
- **core::error::Error always available** - MSRV 1.93 > 1.81 when it was stabilized
- **No breaking changes** - This is a 1.x crate with strict compatibility
