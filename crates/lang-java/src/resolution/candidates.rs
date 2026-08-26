//! What a resolution answers with, and how a staged chain becomes one answer.
//!
//! Typed to `TypeTarget` on purpose. A type can be out of scope or inaccessible,
//! so an answer has to carry its failures alongside its successes. A local in
//! your own scope chain has no such question, and generalising this over §6.1's
//! three namespaces before two of them need it would spread a type parameter
//! for nothing.

use beans_platform_jvm as jvm;

use crate::accessibility::Site;
use crate::model;
use crate::query::Query;

use super::types::InScopeType;

/// The main result of resolving a type name.
///
/// The important thing is that we separate and supply candidates where we can.
/// This allows us to make better decisions and actions when we have multiple candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeResolution {
    /// We couldn't find anything that resembles the type in need.
    /// As a consequence, we cannot supply any supplimental information
    Unresolved {
        invalid_candidates: Vec<InvalidTypeCandidate>,
    },
    /// We had a successful resolution, with a single candidate.
    /// Our happy path.
    Resolved(TypeTarget),
    /// We had a _too_ successful resolution, and we found more than one candidate.
    /// We store all candidates, so we can use this information further down the line.
    Ambiguous(Vec<TypeTarget>),
}

impl TypeResolution {
    pub(crate) fn has_invalidity(&self, reason: TypeInvalidity) -> bool {
        let Self::Unresolved { invalid_candidates } = self else {
            return false;
        };

        invalid_candidates
            .iter()
            .any(|candidate| candidate.has_invalidity(reason))
    }
}

/// Resolution needs to point out what type we resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeTarget {
    /// A file this vertical parsed, so the whole model is in hand and the
    /// declaration is an index into it.
    Parsed {
        source: jvm::model::Source,
        declaration: model::DeclarationId,
    },
    /// Anything but a Java source file, where the lake holds a binary name and
    /// nothing a declaration id could point at.
    Compiled {
        source: jvm::model::Source,
        fqn: jvm::model::BinaryName,
    },
}

impl TypeTarget {
    pub(crate) fn source(&self) -> &jvm::model::Source {
        match self {
            Self::Parsed { source, .. } | Self::Compiled { source, .. } => source,
        }
    }
}

/// Resolution might find candidates, however, that doesn't mean that those candidates are valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TypeInvalidity {
    /// The candidate we found is not in the scope of this module.
    /// Think of this as a `main` module using a class from `test`; normally this direction is set up to be impossible.
    OutsideScope,
    /// The candidate we found is in scope, but its access modifiers prohibit us from using it.
    /// Think of using a private class from another package; resolution sees the class, but it understands that
    Inaccessible,
}

/// Represents a resolution candidate that was rejected for some reason
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InvalidTypeCandidate {
    target: TypeTarget,
    reasons: Vec<TypeInvalidity>,
}

impl InvalidTypeCandidate {
    /// A candidate rejected for one reason. Constructed here rather than by the
    /// namespace that found it, so the invariant that a rejection carries a
    /// reason stays with the type.
    pub(super) fn rejected(target: TypeTarget, reason: TypeInvalidity) -> Self {
        Self {
            target,
            reasons: vec![reason],
        }
    }

    pub(crate) fn target(&self) -> &TypeTarget {
        &self.target
    }

    pub(crate) fn invalidities(&self) -> &[TypeInvalidity] {
        &self.reasons
    }

    pub(crate) fn has_invalidity(&self, reason: TypeInvalidity) -> bool {
        self.invalidities().contains(&reason)
    }

    fn add_reason(&mut self, reason: TypeInvalidity) {
        if !self.has_invalidity(reason) {
            self.reasons.push(reason);
            self.reasons.sort_unstable();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClassifiedTypeCandidate {
    Valid(TypeTarget),
    Invalid(InvalidTypeCandidate),
}

impl ClassifiedTypeCandidate {
    pub(super) fn target(&self) -> &TypeTarget {
        match self {
            Self::Valid(target) => target,
            Self::Invalid(candidate) => candidate.target(),
        }
    }

    pub(super) fn propagate(self, child: ClassifiedTypeCandidate) -> ClassifiedTypeCandidate {
        let Self::Invalid(parent) = self else {
            return child;
        };

        let mut child = match child {
            Self::Valid(target) => InvalidTypeCandidate {
                target,
                reasons: Vec::new(),
            },
            Self::Invalid(candidate) => candidate,
        };
        for reason in parent.reasons {
            child.add_reason(reason);
        }
        Self::Invalid(child)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ResolutionCandidates {
    valid: Vec<TypeTarget>,
    invalid: Vec<InvalidTypeCandidate>,
}

impl ResolutionCandidates {
    pub(super) fn push(&mut self, candidate: ClassifiedTypeCandidate) {
        match candidate {
            ClassifiedTypeCandidate::Valid(target) => {
                if !self.valid.contains(&target) {
                    self.valid.push(target);
                }
            }
            ClassifiedTypeCandidate::Invalid(candidate) => {
                if !self.invalid.contains(&candidate) {
                    self.invalid.push(candidate);
                }
            }
        }
    }

    pub(super) fn into_resolution(self) -> TypeResolution {
        classify_candidates(self.valid, self.invalid)
    }

    pub(super) fn commits_type_path(&self) -> bool {
        self.has_valid()
            || self
                .invalid
                .iter()
                .any(|candidate| candidate.has_invalidity(TypeInvalidity::Inaccessible))
    }

    pub(crate) fn has_valid(&self) -> bool {
        !self.valid.is_empty()
    }

    /// The candidates that survived scope and §6.6.1. Completion offers these
    /// and never the invalid ones: an invalid candidate is evidence for a
    /// diagnostic, not a suggestion.
    pub(crate) fn valid(&self) -> &[TypeTarget] {
        &self.valid
    }

    #[cfg(test)]
    pub(super) fn has_invalidity(&self, reason: TypeInvalidity) -> bool {
        self.invalid
            .iter()
            .any(|candidate| candidate.has_invalidity(reason))
    }
}

impl FromIterator<ClassifiedTypeCandidate> for ResolutionCandidates {
    fn from_iter<T: IntoIterator<Item = ClassifiedTypeCandidate>>(iter: T) -> Self {
        let mut candidates = Self::default();
        for candidate in iter {
            candidates.push(candidate);
        }
        candidates
    }
}

impl IntoIterator for ResolutionCandidates {
    type Item = ClassifiedTypeCandidate;
    type IntoIter = std::vec::IntoIter<ClassifiedTypeCandidate>;

    fn into_iter(self) -> Self::IntoIter {
        self.valid
            .into_iter()
            .map(ClassifiedTypeCandidate::Valid)
            .chain(
                self.invalid
                    .into_iter()
                    .map(ClassifiedTypeCandidate::Invalid),
            )
            .collect::<Vec<_>>()
            .into_iter()
    }
}

fn classify_candidates(
    candidates: impl IntoIterator<Item = TypeTarget>,
    invalid_candidates: Vec<InvalidTypeCandidate>,
) -> TypeResolution {
    let mut distinct = Vec::new();
    for candidate in candidates {
        if !distinct.contains(&candidate) {
            distinct.push(candidate);
        }
    }

    match distinct.len() {
        0 => TypeResolution::Unresolved { invalid_candidates },
        1 => TypeResolution::Resolved(distinct.pop().unwrap()),
        _ => TypeResolution::Ambiguous(distinct),
    }
}

/// §6.4.1 as a fold: the first stage to produce a valid candidate answers, and
/// the invalid ones passed over on the way are kept.
///
/// Keeping them is what javac does and what recovery needs — an inaccessible
/// import is evidence worth reporting, and it must not hide an accessible
/// answer from a later stage. It is also why this cannot be `find`.
pub(crate) fn first_stage_that_answers(
    candidates: impl Iterator<Item = InScopeType>,
    from: &Site,
    query: &Query,
) -> ResolutionCandidates {
    let mut answered = ResolutionCandidates::default();
    let mut stage = None;

    for candidate in candidates {
        if stage != Some(candidate.stage) {
            if answered.has_valid() {
                break;
            }
            stage = Some(candidate.stage);
        }
        answered.push(candidate.classify(query, from));
    }

    answered
}
