use beans_core_engine::Revision;
use beans_core_model::source::Source;
use beans_lang_java_model::{
    File,
    imports::ImportType,
    names::Name,
    nodes::{NodeIndex, NodeKind, types::Kind},
    references::{TypeBound, TypeNameComponent, TypeRef},
};

pub use crate::query::JavaDeclarationHandle as DeclarationHandle;
use crate::query::{JavaQuery, JavaTypeEntry};

mod access;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedDeclaration {
    Java {
        declaration: DeclarationHandle,
    },
    Jvm {
        revision: Revision,
        // TODO: we don't know these yet
    },
    Parameter {
        declaration: DeclarationHandle,
        name: String,
    },
}

pub type ResolvedTypeArgument = TypeBound<Vec<ResolutionResult>>;

#[derive(Debug, Clone)]
pub struct ResolvedType {
    pub declaration: ResolvedDeclaration,
    pub arguments: Vec<ResolvedTypeArgument>,
}

#[derive(Debug, Clone)]
pub enum ResolutionResult {
    Resolved(ResolvedType),
    Ambiguous(Vec<ResolvedType>),
    NotFound,
    /// Known candidates are retained, but lookup could not establish a usable binding.
    Blocked {
        candidates: Vec<ResolvedType>,
        problems: Vec<LookupProblem>,
    },
}

#[derive(Debug, Clone)]
pub enum LookupProblem {
    Supertype {
        owner: DeclarationHandle,
        reference: TypeRef,
        results: Vec<ResolutionResult>,
    },
    Cycle {
        owner: DeclarationHandle,
        name: String,
    },
    Inaccessible(DeclarationHandle),
    InvalidImport(Name),
    UnavailableModel,
    Unsupported(&'static str),
}

#[derive(Debug, Default)]
struct TypeLookup {
    declarations: Vec<ResolvedDeclaration>,
    problems: Vec<LookupProblem>,
}

impl TypeLookup {
    fn is_empty(&self) -> bool {
        self.declarations.is_empty() && self.problems.is_empty()
    }

    fn blocked(problem: LookupProblem) -> Self {
        Self {
            declarations: Vec::new(),
            problems: vec![problem],
        }
    }
}

/// Body references use their occurrence node; declaration references use their owning type node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceLocation {
    Body,
    Supertype,
    SupertypeArgument,
    TypeParameterBound,
}

type ActiveLookups = Vec<(DeclarationHandle, String)>;

pub struct ResolverContext<'a> {
    source: &'a Source,
    node_index: NodeIndex,
    type_ref: &'a TypeRef,
    location: ReferenceLocation,
    query: &'a JavaQuery<'a>,
}

impl<'a> ResolverContext<'a> {
    pub fn new(
        source: &'a Source,
        node_index: NodeIndex,
        type_ref: &'a TypeRef,
        location: ReferenceLocation,
        query: &'a JavaQuery<'a>,
    ) -> Self {
        Self {
            source,
            node_index,
            type_ref,
            location,
            query,
        }
    }
}

pub struct Resolver {}

impl Resolver {
    pub fn resolve(&self, ctx: &ResolverContext<'_>) -> Vec<ResolutionResult> {
        self.resolve_reference(ctx, &mut Vec::new())
    }

    fn resolve_reference(
        &self,
        ctx: &ResolverContext<'_>,
        active: &mut ActiveLookups,
    ) -> Vec<ResolutionResult> {
        // If we are not resolving a named type, what are we even doing here?!
        let TypeRef::Named { segments } = ctx.type_ref else {
            return vec![];
        };

        let mut results = Vec::with_capacity(segments.len());
        // Qualified types like `OtherFile.Inner` can escape our current file
        // This state keeps track which file, which declaration we resolved our last component, so we can continue
        let mut resolution_base = None;

        // We take each segment of a (possibly) complex type like `Outer<String>.Inner<T>`, and progressively resolve them from left to right
        for segment in segments {
            let member_base = match resolution_base.as_ref() {
                Some(ResolvedDeclaration::Java { declaration }) => Some(declaration),
                Some(base) => panic!("member lookup is not implemented for {base:?}"),
                None => None,
            };

            // We resolve the type arguments first
            // Note that argument resolution is not moving based on `resolution_base`. It's bound to the starting declaration
            let arguments = self.resolve_type_arguments(ctx, segment, active);

            // The big one: we go, and find all declarations matching the name we are on
            let lookup = match member_base {
                Some(base) => {
                    let members = self.lookup_member_types(ctx, base, &segment.name, active);
                    self.check_access(ctx, members)
                }
                None => self.resolve_type_declarations(ctx, &segment.name, active),
            };
            let TypeLookup {
                declarations: mut candidates,
                problems,
            } = lookup;

            let result = if !problems.is_empty() {
                ResolutionResult::Blocked {
                    candidates: candidates
                        .into_iter()
                        .map(|declaration| ResolvedType {
                            declaration,
                            arguments: arguments.clone(),
                        })
                        .collect(),
                    problems,
                }
            } else {
                match candidates.len() {
                    0 => ResolutionResult::NotFound,
                    1 => ResolutionResult::Resolved(ResolvedType {
                        declaration: candidates.pop().expect("exactly one candidate"),
                        arguments,
                    }),
                    _ => ResolutionResult::Ambiguous(
                        candidates
                            .into_iter()
                            .map(|declaration| ResolvedType {
                                declaration,
                                arguments: arguments.clone(),
                            })
                            .collect(),
                    ),
                }
            };

            match result {
                ResolutionResult::Resolved(resolved) => {
                    resolution_base = Some(resolved.declaration.clone());
                    results.push(ResolutionResult::Resolved(resolved));
                }
                failure => {
                    results.push(failure);
                    break;
                }
            }
        }

        results
    }

    /// JLS §4.5.1: retain the argument shape; resolve its references at the use site.
    fn resolve_type_arguments(
        &self,
        ctx: &ResolverContext<'_>,
        segment: &TypeNameComponent,
        active: &mut ActiveLookups,
    ) -> Vec<ResolvedTypeArgument> {
        let location = match ctx.location {
            ReferenceLocation::Supertype => ReferenceLocation::SupertypeArgument,
            location => location,
        };
        let mut resolve_reference = |type_ref: &TypeRef| {
            let argument_ctx =
                ResolverContext::new(ctx.source, ctx.node_index, type_ref, location, ctx.query);
            self.resolve_reference(&argument_ctx, active)
        };

        segment
            .bounds
            .iter()
            .map(|bound| bound.map_ref(&mut resolve_reference))
            .collect()
    }

    /// JLS §6.5.5.1: resolves an unqualified type component from its use site.
    fn resolve_type_declarations(
        &self,
        ctx: &ResolverContext<'_>,
        name: &str,
        active: &mut ActiveLookups,
    ) -> TypeLookup {
        let lookup = self.lookup_lexical_types(ctx, name, active);
        if !lookup.is_empty() {
            return lookup;
        }
        let lookup = self.lookup_imported_types(ctx, name);
        if !lookup.is_empty() {
            return lookup;
        }

        let Some(file) = ctx.query.file(ctx.source) else {
            return TypeLookup::blocked(LookupProblem::UnavailableModel);
        };
        let lookup = self.lookup_package_type(ctx, &file.package_name, name);
        if !lookup.is_empty() {
            return lookup;
        }
        if file.imports.iter().any(|import| {
            matches!(
                import.typ(),
                ImportType::OnDemandType
                    | ImportType::OnDemandStaticType
                    | ImportType::SingleModule
            )
        }) {
            return TypeLookup::blocked(LookupProblem::Unsupported(
                "On-demand and module imports are not implemented",
            ));
        }
        self.lookup_package_type(ctx, &Name::new(vec!["java".into(), "lang".into()]), name)
    }

    /// JLS §6.3–§6.4.1: scope depends on the reference's location, not containment alone.
    fn lookup_lexical_types(
        &self,
        ctx: &ResolverContext<'_>,
        name: &str,
        active: &mut ActiveLookups,
    ) -> TypeLookup {
        let Some(file) = ctx.query.file(ctx.source) else {
            return TypeLookup::blocked(LookupProblem::UnavailableModel);
        };
        if file.node(ctx.node_index).is_none() {
            return TypeLookup::blocked(LookupProblem::UnavailableModel);
        }

        for entry in file.iter_ancestors(ctx.node_index) {
            match entry.node.kind() {
                NodeKind::Type(declaration) => {
                    let owner = ctx.query.declaration_handle(ctx.source, entry.index);
                    let inside_body =
                        entry.index != ctx.node_index || ctx.location == ReferenceLocation::Body;
                    if inside_body {
                        let declarations =
                            self.find_declared_types(ctx, file, ctx.source, entry.index, name);
                        if !declarations.is_empty() {
                            return TypeLookup {
                                declarations,
                                problems: Vec::new(),
                            };
                        }
                    }
                    if let Some(parameter) = declaration.type_parameter_named(name) {
                        if entry.index == ctx.node_index
                            && ctx.location == ReferenceLocation::Supertype
                        {
                            return TypeLookup::blocked(LookupProblem::Unsupported(
                                "Type-parameter names in supertype positions need scope validation",
                            ));
                        }
                        return TypeLookup {
                            declarations: vec![ResolvedDeclaration::Parameter {
                                declaration: owner,
                                name: parameter.name.clone(),
                            }],
                            problems: Vec::new(),
                        };
                    }
                    if inside_body {
                        let lookup = self.lookup_member_types(ctx, &owner, name, active);
                        if !lookup.is_empty() {
                            return self.check_access(ctx, lookup);
                        }
                    }
                }
                NodeKind::CompilationUnit => {
                    return TypeLookup {
                        declarations: self.find_declared_types(
                            ctx,
                            file,
                            ctx.source,
                            entry.index,
                            name,
                        ),
                        problems: Vec::new(),
                    };
                }
                NodeKind::Field(_) => {}
                NodeKind::Method(_) | NodeKind::Block => {
                    return TypeLookup::blocked(LookupProblem::Unsupported(
                        "Method and occurrence-sensitive block scopes are not implemented",
                    ));
                }
            }
        }
        TypeLookup::default()
    }

    fn find_declared_types(
        &self,
        ctx: &ResolverContext<'_>,
        file: &File,
        source: &Source,
        node: NodeIndex,
        name: &str,
    ) -> Vec<ResolvedDeclaration> {
        let mut declarations = Vec::new();
        for child in file.iter_children(node) {
            let NodeKind::Type(declaration) = child.node.kind() else {
                continue;
            };
            if declaration.name.as_deref() == Some(name) {
                declarations.push(ResolvedDeclaration::Java {
                    declaration: ctx.query.declaration_handle(source, child.index),
                });
            }
        }
        declarations
    }

    /// JLS §8.5, §9.5: declared members hide inherited members of the same name.
    fn lookup_member_types(
        &self,
        ctx: &ResolverContext<'_>,
        owner: &DeclarationHandle,
        name: &str,
        active: &mut ActiveLookups,
    ) -> TypeLookup {
        let Some(entry) = ctx.query.declaration(owner) else {
            return TypeLookup::blocked(LookupProblem::UnavailableModel);
        };
        let declarations =
            self.find_declared_types(ctx, entry.file, entry.source, entry.node_index, name);
        if !declarations.is_empty() {
            return TypeLookup {
                declarations,
                problems: Vec::new(),
            };
        }
        if active
            .iter()
            .any(|(base, member)| base == owner && member == name)
        {
            return TypeLookup::blocked(LookupProblem::Cycle {
                owner: owner.clone(),
                name: name.to_owned(),
            });
        }

        active.push((owner.clone(), name.to_owned()));
        let lookup = self.lookup_inherited_types(ctx, owner, entry, name, active);
        active.pop();
        lookup
    }

    fn lookup_inherited_types(
        &self,
        ctx: &ResolverContext<'_>,
        owner: &DeclarationHandle,
        entry: JavaTypeEntry<'_>,
        name: &str,
        active: &mut ActiveLookups,
    ) -> TypeLookup {
        let mut lookup = TypeLookup::default();
        let supertypes = entry
            .declaration
            .declared_superclass
            .iter()
            .chain(&entry.declaration.declared_superinterfaces);

        for reference in supertypes {
            let supertype_ctx = ResolverContext::new(
                entry.source,
                entry.node_index,
                reference,
                ReferenceLocation::Supertype,
                ctx.query,
            );
            let results = self.resolve_reference(&supertype_ctx, active);
            let Some(ResolutionResult::Resolved(resolved)) = results.last() else {
                lookup.problems.push(LookupProblem::Supertype {
                    owner: owner.clone(),
                    reference: reference.clone(),
                    results,
                });
                continue;
            };
            let ResolvedDeclaration::Java { declaration } = &resolved.declaration else {
                lookup.problems.push(LookupProblem::Unsupported(
                    "A supertype must resolve to a Java class or interface",
                ));
                continue;
            };

            let inherited = self.lookup_member_types(ctx, declaration, name, active);
            lookup.problems.extend(inherited.problems);
            for candidate in inherited.declarations {
                let ResolvedDeclaration::Java { declaration } = &candidate else {
                    unreachable!("member lookup only produces class/interface declarations");
                };
                match access::is_inheritable(ctx.query, declaration, entry) {
                    Ok(true) => {
                        if !lookup.declarations.contains(&candidate) {
                            lookup.declarations.push(candidate);
                        }
                    }
                    Ok(false) => {}
                    Err(problem) => lookup.problems.push(problem),
                }
            }
        }

        if matches!(entry.declaration.kind, Kind::Enum | Kind::Record) {
            lookup.problems.push(LookupProblem::Unsupported(
                "Implicit enum and record supertypes are not implemented",
            ));
        }
        lookup
    }

    /// JLS §7.5.1: repeated imports of one declaration do not introduce ambiguity.
    fn lookup_imported_types(&self, ctx: &ResolverContext<'_>, name: &str) -> TypeLookup {
        let Some(file) = ctx.query.file(ctx.source) else {
            return TypeLookup::blocked(LookupProblem::UnavailableModel);
        };
        let mut lookup = TypeLookup::default();
        for import in &file.imports {
            if !import.is_name_valid()
                || import.name().as_slice().last().map(String::as_str) != Some(name)
            {
                continue;
            }
            if import.typ() == ImportType::SingleStaticType {
                lookup.problems.push(LookupProblem::Unsupported(
                    "Static imports are not implemented",
                ));
                continue;
            }
            if import.typ() != ImportType::SingleType {
                continue;
            }
            let targets = ctx.query.find_type(import.name());
            if targets.is_empty() {
                lookup
                    .problems
                    .push(LookupProblem::InvalidImport(import.name().clone()));
            }
            for target in targets {
                if target.file.package_name.is_empty() {
                    lookup
                        .problems
                        .push(LookupProblem::InvalidImport(import.name().clone()));
                    continue;
                }
                let declaration = ctx
                    .query
                    .declaration_handle(target.source, target.node_index);
                for ancestor in target.file.iter_ancestors(target.node_index) {
                    if !matches!(ancestor.node.kind(), NodeKind::Type(_)) {
                        continue;
                    }
                    let enclosing = ctx.query.declaration_handle(target.source, ancestor.index);
                    match access::is_accessible(ctx, &enclosing) {
                        Ok(true) => {}
                        Ok(false) => lookup.problems.push(LookupProblem::Inaccessible(enclosing)),
                        Err(problem) => lookup.problems.push(problem),
                    }
                }
                let candidate = ResolvedDeclaration::Java { declaration };
                if !lookup.declarations.contains(&candidate) {
                    lookup.declarations.push(candidate);
                }
            }
        }
        lookup
    }

    fn lookup_package_type(
        &self,
        ctx: &ResolverContext<'_>,
        package: &Name,
        name: &str,
    ) -> TypeLookup {
        let canonical = package
            .as_slice()
            .iter()
            .cloned()
            .chain(std::iter::once(name.to_owned()))
            .collect();
        let declarations = ctx
            .query
            .find_type(&canonical)
            .into_iter()
            .filter(|entry| &entry.file.package_name == package)
            .map(|entry| ResolvedDeclaration::Java {
                declaration: ctx.query.declaration_handle(entry.source, entry.node_index),
            })
            .collect();
        self.check_access(
            ctx,
            TypeLookup {
                declarations,
                problems: Vec::new(),
            },
        )
    }

    fn check_access(&self, ctx: &ResolverContext<'_>, mut lookup: TypeLookup) -> TypeLookup {
        for candidate in &lookup.declarations {
            let ResolvedDeclaration::Java { declaration } = candidate else {
                continue;
            };
            match access::is_accessible(ctx, declaration) {
                Ok(true) => {}
                Ok(false) => lookup
                    .problems
                    .push(LookupProblem::Inaccessible(declaration.clone())),
                Err(problem) => lookup.problems.push(problem),
            }
        }
        lookup
    }
}

#[cfg(test)]
mod prototype_tests;
