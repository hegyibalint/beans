use crate::model::{imports::ImportType, references::TypeNameComponent};
use beans_core::model::{names::Name, ranges::Spanned};

use super::super::{
    Context, ResolutionFailure, TypeCandidate, query::TypeCandidateQuery,
    result::classify_candidates,
};

/// Resolves names outside the current compilation unit through the visible definitions.
/// The query searches Java and JVM definitions together; lookup decides JLS precedence.
pub(in crate::semantics::resolution) fn lookup_external(
    ctx: &Context<'_>,
    segments: &[Spanned<TypeNameComponent>],
) -> Result<Option<TypeCandidate>, ResolutionFailure> {
    let Some(definitions) = ctx.definitions else {
        return Ok(None);
    };
    let Some((first, suffix)) = segments.split_first() else {
        return Err(ResolutionFailure::InvalidTypeRef);
    };
    let simple_name = &first.value().name;
    let first_segment = std::slice::from_ref(first);

    // JLS §6.4.1: a single-type import shadows types in other files of the
    // current package and types imported on demand.
    let single_imports = ctx
        .file
        .imports
        .iter()
        .filter(|import| import.typ() == ImportType::SingleType && import.is_name_valid())
        .filter(|import| import.name().as_slice().last() == Some(simple_name))
        .map(|import| import.name().clone());
    if let Some(candidate) = lookup_paths(definitions, single_imports, suffix)? {
        return Ok(Some(candidate));
    }

    let same_package = append(ctx.file.package_name.as_slice(), first_segment);
    if let Some(candidate) = lookup_paths(definitions, std::iter::once(same_package), suffix)? {
        return Ok(Some(candidate));
    }

    // JLS §7.3 implicitly imports java.lang.* into every compilation unit.
    let on_demand = ctx
        .file
        .imports
        .iter()
        .filter(|import| import.typ() == ImportType::OnDemandType && import.is_name_valid())
        .map(|import| append(import.name().as_slice(), first_segment))
        .chain(std::iter::once(append(
            &["java".into(), "lang".into()],
            first_segment,
        )));
    if let Some(candidate) = lookup_paths(definitions, on_demand, suffix)? {
        return Ok(Some(candidate));
    }

    // A qualified spelling can start with a package rather than an imported type
    // (JLS §6.5.5.2). Only the complete canonical type name can be queried here.
    if segments.len() > 1 {
        let written = append(&[], segments);
        if let Some(candidate) = classify_candidates(definitions.candidates(&written).into_iter())?
        {
            return Ok(Some(candidate));
        }
        for prefix_len in 1..segments.len() {
            let prefix = append(&[], &segments[..prefix_len]);
            if classify_candidates(definitions.candidates(&prefix).into_iter())?.is_some() {
                return Err(ResolutionFailure::Partial);
            }
        }
    }

    Ok(None)
}

/// A bound prefix commits to this tier even if its qualified suffix is missing.
fn lookup_paths(
    definitions: &dyn TypeCandidateQuery,
    paths: impl Iterator<Item = Name>,
    suffix: &[Spanned<TypeNameComponent>],
) -> Result<Option<TypeCandidate>, ResolutionFailure> {
    let mut owners = Vec::<(Name, TypeCandidate)>::new();
    for path in paths {
        for candidate in definitions.candidates(&path) {
            if !owners.iter().any(|(_, known)| known == &candidate) {
                owners.push((path.clone(), candidate));
            }
        }
    }

    // JLS §6.5.5.1 requires a unique first type. A suffix cannot choose between
    // two imported types with the same simple name.
    let Some(owner) = classify_candidates(owners.iter().map(|(_, candidate)| candidate.clone()))?
    else {
        return Ok(None);
    };
    if suffix.is_empty() {
        return Ok(Some(owner));
    }
    let (path, _) = owners
        .iter()
        .find(|(_, candidate)| candidate == &owner)
        .expect("classified owner must have a path");
    let qualified = append(path.as_slice(), suffix);
    let member = classify_candidates(definitions.candidates(&qualified).into_iter())?
        .ok_or(ResolutionFailure::Partial)?;
    Ok(Some(member))
}

fn append(prefix: &[String], segments: &[Spanned<TypeNameComponent>]) -> Name {
    prefix
        .iter()
        .cloned()
        .chain(segments.iter().map(|segment| segment.value().name.clone()))
        .collect()
}
