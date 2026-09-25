//! Java type-name resolution.
//!
//! # Resolution pipeline
//!
//! A named type reference is resolved component by component. `lookup_upward` resolves the first
//! component relative to the occurrence. After selecting a type prefix, `lookup_downward` resolves
//! every remaining component only below that selected owner. A failed suffix never restarts lookup
//! in another lexical scope. These rules follow the distinction between simple and qualified type
//! names in JLS §6.5.5.1 and §6.5.5.2.
//!
//! The stages below are the semantic contract for this module. Some compilation-unit stages are
//! not wired into `resolve` yet.
//!
//! ## 1. Validate the reference
//!
//! `resolve` currently accepts a nonempty `TypeRef::Named`. Other reference shapes are resolved by
//! their respective type rules rather than by type-name lookup.
//!
//! ## 2. Resolve the first component in a type body
//!
//! Lexical type scopes are visited from the occurrence outwards. For each enclosing type, the
//! recovery precedence is:
//!
//! 1. directly declared member types;
//! 2. type parameters;
//! 3. inherited member types;
//! 4. if none matched, the next enclosing type.
//!
//! The first enclosing scope that produces candidates wins; candidates from different enclosing
//! scopes are not combined. Scope and type-declaration shadowing are specified by JLS §6.3 and
//! §6.4.1. Declared member types hide inherited member types according to JLS §8.5 and §9.5.
//! JLS §6.5.5.1 requires exactly one in-scope declaration; the ordering above also gives malformed
//! or incomplete source a deterministic recovery result.
//!
//! Method and constructor type parameters, local type declarations, and block scopes belong before
//! the enclosing-type stages when the Java model represents them.
//!
//! ## 3. Resolve the first component in a supertype clause
//!
//! `resolve_supertype` uses a different first scope:
//!
//! 1. type parameters of the type being declared;
//! 2. skip that type's declared and inherited members;
//! 3. visit each enclosing type using the normal body precedence from stage 2.
//!
//! A member's scope is its declaring type's body, while a class or interface type parameter is also
//! in scope in its superclass and superinterface clauses (JLS §6.3). This is why ordinary body
//! resolution must not be used to resolve a direct supertype edge.
//!
//! ## 4. Resolve compilation-unit names
//!
//! After upward lookup is exhausted, downward lookup from the compilation-unit root resolves
//! top-level types declared in the current file. Package and import lookup are not wired yet.
//! Subject to the declaration-conflict rules in JLS §7.5.1 and §7.5.3, the remaining shadowing
//! tiers are:
//!
//! 1. single-type and single-static type imports;
//! 2. top-level types in the current package;
//! 3. type-import-on-demand and static-import-on-demand declarations, including the implicit
//!    `java.lang.*` import;
//! 4. single-module-import declarations.
//!
//! A single import shadows same-named types declared in other compilation units of the current
//! package and types supplied by on-demand or module imports. Current-package types shadow
//! on-demand imports, while ordinary and static on-demand imports shadow module imports. The exact
//! shadowing rules are in JLS §6.4.1; import forms and their conflict rules are in JLS §7.5.1–§7.5.5.
//! Top-level type scope is defined by JLS §6.3 and §7.6.
//!
//! Fully qualified names and names expanded from imports are discovered through
//! `TypeDefinitionQuery`; package/type boundaries are not inferred from capitalization.
//!
//! ## 5. Resolve qualified suffixes downward
//!
//! Once the first component selects a single type candidate, each remaining component is resolved
//! below that owner:
//!
//! 1. directly declared member types;
//! 2. inherited member types;
//! 3. otherwise the reference is only partially resolved.
//!
//! Downward lookup never considers type-parameter declarations. When upward lookup selects a type
//! parameter with a remaining suffix, it substitutes each bound at the parameter's declaration
//! point and recursively resolves the resulting path. This follows the intersection-type member
//! rule in JLS §4.4. Member accessibility and the requirement for exactly one accessible member are
//! specified by JLS §6.5.5.2 and §6.6.1.
//!
//! ## 6. Inherited member types
//!
//! Inherited lookup resolves every direct superclass and superinterface edge, in declaration order.
//! For each supertype branch, a directly declared matching member hides matching declarations above
//! it; otherwise lookup continues recursively through that supertype's own direct supertypes. A
//! non-inheritable declaration still stops its branch because hiding and inheritance are distinct.
//! The inheritance and ambiguity rules are specified by JLS §8.5 and §9.5.
//!
//! Results preserve first-discovery order. The same declaration reached through multiple paths is
//! included once by `TypeCandidate` identity; distinct declarations remain distinct and therefore
//! ambiguous. Traversal must also detect cyclic inheritance independently of result deduplication.
//!
//! ## 7. Classify the result
//!
//! No candidates means lookup continues to the next permitted stage or reports `NotFound`. One
//! distinct candidate produces a `TypeCandidate`; multiple distinct candidates produce
//! `ResolutionFailure::Ambiguous` in deterministic discovery order. Accessibility can invalidate
//! a candidate but must not cause lookup to fall back to a scope shadowed by the selected
//! declaration.
//!
//! ## 8. Validate the selected meaning
//!
//! After lookup resolves the name, validation checks whether the selected declaration may be used
//! at this occurrence. A class or interface type parameter remains in lexical scope across a static
//! nesting boundary, but using it there can be illegal under JLS §6.5.5.1. Validation therefore
//! turns the successful binding into `ResolutionFailure::IllegalTypeParameterUse` rather than
//! retrying lookup in another scope.

mod iterators;
mod lookup;
pub mod query;
mod resolve;
mod result;
mod validation;

#[cfg(test)]
mod tests;

pub use self::{
    resolve::{resolve, resolve_supertype},
    result::{JavaTypeCandidate, ResolutionFailure, TypeCandidate, TypeParameterHandle},
};
use crate::model::{File, nodes::NodeIndex, references::TypeRef};
use beans_core::engine::Revision;
use beans_core::model::source::Source;

pub struct Context<'a> {
    revision: Revision,
    source: &'a Source,
    file: &'a File,
    node_index: NodeIndex,
    type_ref: &'a TypeRef,
}

impl<'a> Context<'a> {
    pub fn new(
        revision: Revision,
        source: &'a Source,
        file: &'a File,
        node_index: NodeIndex,
        type_ref: &'a TypeRef,
    ) -> Self {
        Self {
            revision,
            source,
            file,
            node_index,
            type_ref,
        }
    }
}
