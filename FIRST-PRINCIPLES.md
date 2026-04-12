# First Principles

## Principle 1: Test Driven Development

**Philosophy**: Systematic TDD - write unit test immediately after each logical unit of code.

**Implementation implications**:

- Each logical unit (function, object, method) needs at least one unit test
- Cover edge cases and validation errors (multiple tests per unit)
- Run tests after atomic changes; amend tests only after first run
- Use LSP to identify calling/called code relationships
- When testing for scenarios involving the filesystem (access rights, missing file or folder) always use mockup tests and never manipulate the filesystem itself

## Principle 2: Leverage Existing Solutions

**Philosophy**: Reuse mature, well-maintained libraries rather than reinventing functionality.

**Implementation implications**:

- Prefer established, actively maintained libraries with strong community support
- Choose mature solutions with proven track record (but not stale/unmaintained)
- Follow standard protocols and interfaces when available
- Ensure compatibility with existing toolchains and ecosystems
- Evaluate library health: recent updates, active issues/PRs, documentation quality
- Align with industry best practices and conventions

## Principle 3: Compile Once, Evaluate Many

**Philosophy**: Separate expensive analysis from cheap repeated computation.

**Implementation implications**:

- Validation and constant folding happen at compile time, never at eval time
- The CompiledExpr IR is optimized for fast traversal, not for readability
- Argument binding uses positional indices, not string lookups at eval time
- Sum/product index variables use a separate index space from arguments
- Any work that can be done once should be done in the compile phase

## Principle 4: Transparent Numeric Promotion

**Philosophy**: Real and complex numbers coexist seamlessly with automatic promotion.

**Implementation implications**:

- Real operations on real inputs produce real results (no unnecessary complex wrapping)
- Complex results that have negligible imaginary parts simplify back to real
- The `is_complex` flag on CompiledExpr is a compile-time hint, not a runtime constraint
- Domain-extending functions (sqrt of negative, ln of negative) promote automatically
- Users never need to declare complex mode; it emerges from the data

## Principle 5: Broadcasting as Cartesian Product

**Philosophy**: Multi-argument evaluation follows Cartesian product semantics over arrays.

**Implementation implications**:

- Each array argument contributes one axis to the output shape
- Scalar arguments broadcast to all positions without adding axes
- Axis ordering follows argument declaration order (first appearance in AST)
- Row-major output layout matches ndarray defaults
- Per-element errors do not fail the entire batch

## Principle 6: Lazy by Default

**Philosophy**: Defer computation until the caller explicitly requests results.

**Implementation implications**:

- `eval()` returns a handle, not results — no computation runs at call time
- Callers choose their consumption mode: scalar, eager array, or lazy iterator
- Shape information is available before any element is computed
- Iterator consumption enables streaming without materializing the full output
- This design enables future optimizations (e.g., early termination, partial evaluation)
