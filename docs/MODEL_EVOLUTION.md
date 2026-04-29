# Model Evolution Tracker

Tracks remaining work on the beans-core data model, identified through systematic JLS analysis (Chapters 4-16).

## Status Legend
- **done** — merged and tested
- **todo** — identified, not started
- **blocked** — depends on another item

---

## Tier 1: Foundation (done)

These are in the codebase now.

| Item | File | Status |
|------|------|--------|
| `TypeRef` enum | `type_ref.rs` | done |
| `TypeParam` struct | `type_ref.rs` | done |
| `PrimitiveKind` with widening + boxing | `type_ref.rs` | done |
| `TypeRef::erasure()` | `type_ref.rs` | done |
| `TypeRef::substitute()` | `type_ref.rs` | done |
| `SymbolKind::EnumConstant` | `symbol_kind.rs` | done |
| `Modifier::NonSealed` | `modifier.rs` | done |
| `RelationKind::Permits` | `relation.rs` | done |
| `Relation.type_args: Vec<TypeRef>` | `relation.rs` | done |

---

## Tier 2: Signature migration (todo)

Migrate `Signature` fields from `String` to `TypeRef`/`TypeParam`. This is the highest-impact remaining work — it connects `TypeRef` to the rest of the system.

| Item | Current | Target | Blocks |
|------|---------|--------|--------|
| `Method.return_type` | `String` | `TypeRef` | type checking, hover |
| `Method.parameters[].param_type` | `String` | `TypeRef` | overload resolution |
| `Method.type_parameters` | `Vec<String>` | `Vec<TypeParam>` | generic method resolution |
| `Method.throws` | **missing** | `Vec<TypeRef>` | checked exception diagnostics |
| `MethodParam.is_varargs` | **missing** | `bool` | 3-phase overload resolution |
| `Field.field_type` | `String` | `TypeRef` | assignment checking |
| `Field.constant_value` | **missing** | `Option<ConstantValue>` | switch duplicates, annotation values |
| `Field.initialized` | **missing** | `bool` | blank final / definite assignment |
| `Class.type_parameters` | `Vec<String>` | `Vec<TypeParam>` | generic class resolution |

### New Signature variants (todo)

| Variant | Fields | Blocks |
|---------|--------|--------|
| `Record { type_parameters: Vec<TypeParam>, components: Vec<RecordComponent> }` | ordered component list | pattern matching, accessor synthesis |
| `AnnotationElement { element_type: TypeRef, default_value: Option<ConstantValue> }` | annotation-specific | annotation validation, completion |

### Supporting types needed (todo)

| Type | Fields | Used by |
|------|--------|---------|
| `RecordComponent { name: String, component_type: TypeRef }` | | `Signature::Record` |
| `ConstantValue` enum: `Int(i64), Float(f64), Str(String), Bool(bool), Char(char), Null` | | `Signature::Field`, `Signature::AnnotationElement` |

---

## Tier 3: Symbol extensions (todo)

| Item | Where | Purpose |
|------|-------|---------|
| `annotations: Vec<AnnotationInstance>` | `Symbol` struct | meta-annotations (@Retention, @Target, @Repeatable) |
| `AnnotationInstance { fqn: String, elements: Vec<(String, AnnotationValue)> }` | new struct | annotation data model |
| `AnnotationValue` enum | new struct | `Const(ConstantValue), ClassLiteral(TypeRef), EnumRef(String, String), Annotation(AnnotationInstance), Array(Vec<AnnotationValue>)` |

---

## Tier 4: SymbolTable fixes (deferred → Tier 6)

| Item | Current | Fix | Impact |
|------|---------|-----|--------|
| FQN overload collision | `HashMap<String, SymbolId>` | `HashMap<String, Vec<SymbolId>>` | overloaded methods silently dropped |

---

## Tier 5: Algorithms (todo)

These are computations over the model, not data model changes. They depend on Tier 2.

| Algorithm | Purpose | Depends on |
|-----------|---------|------------|
| Inherited member resolution | Walk supertype DAG, collect members with visibility/hiding/override rules | Relation.type_args, TypeRef |
| Type substitution (cross-file) | `Signature[T := String]` through inheritance chains | TypeRef, TypeParam |
| Overload resolution (3-phase) | Strict → boxing → varargs method selection | TypeRef, is_varargs, widening |
| SAM detection | Is this interface a @FunctionalInterface? | Inherited member resolution |
| LUB computation | Least upper bound for ternary, multi-catch | SupertypeRegistry, TypeRef |
| Exhaustiveness checking | Sealed type switch coverage | Permits relation |
| Synthetic member generation | Enum values()/valueOf(), record accessors, default constructors | Signature::Record, EnumConstant |
| Wildcard import ambiguity | Detect conflicting wildcard imports | PackageRegistry |
| Access control enforcement | Check modifiers during resolution | Modifier, PackageRegistry |

---

## Tier 6: Semantic graph (todo)

The pull-based computation graph with push-invalidation, serialization, and warm restart. This is the architecture for diagnostics and type checking.

| Item | Purpose |
|------|---------|
| Graph node model (`CacheNode`) | Nodes with cached values, cache state, layer |
| Hard edges (ownership trees) | File → CST → Symbols, parent-child |
| Soft edges (registry queries) | Cross-file dependencies via SymbolRegistry, SupertypeRegistry, PackageRegistry |
| Registry pattern | Producers register, consumers subscribe, reconnection on delete/restore |
| Push-stale invalidation | Tree-sitter diff → mark affected nodes stale upward |
| Pull-recompute on demand | LSP request → pull from top, short-circuit at fresh nodes |
| Serialization / warm restart | Binary graph snapshot, instant startup, background diff |
| Diagnostic rule trait | `check(&self, ctx: &RuleContext) -> Vec<Diagnostic>` |
| ModuleRegistry | JPMS: exports, requires, opens, qualified exports, transitive requires |

---

## Parser work (todo)

The Java parser needs to produce `TypeRef` instead of `String`. This connects the model to actual source files.

| Item | Current | Target |
|------|---------|--------|
| Parse type references as `TypeRef` | String extraction | Structured `TypeRef` from tree-sitter type nodes |
| Parse type parameter bounds | `Vec<String>` | `Vec<TypeParam>` with bounds |
| Parse `throws` clauses | not parsed | `Vec<TypeRef>` |
| Parse varargs | not tracked | `is_varargs: bool` on MethodParam |
| Parse `permits` clauses | not parsed | `Relation { kind: Permits, ... }` |
| Parse `non-sealed` modifier | not parsed | `Modifier::NonSealed` |
| Parse enum constants as `EnumConstant` | parsed as `Field` | `SymbolKind::EnumConstant` |
| Parse record components | not parsed | `Signature::Record { components }` |
| Parse annotations on symbols | not parsed | `Vec<AnnotationInstance>` |
| Parse constant initializers | not parsed | `ConstantValue` on Field |
