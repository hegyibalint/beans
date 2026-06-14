//! Parse → commit handoff: [`IntegrationJob`].
//!
//! Bulk indexing fans parsing out across rayon workers (ADR-0005) and
//! then commits the results serially on the graph thread (ADR-0018). The
//! seam between those two phases is one owned, `Send` value per file: a
//! worker parses source into it; the graph thread consumes it to mutate
//! the graph and registries.
//!
//! `IntegrationJob` is that seam as a trait, so a heterogeneous batch of
//! jobs from different language verticals can be committed through one
//! loop without the committer naming each concrete parsed-file type. The
//! engine owns the trait because it owns the graph the job integrates
//! into; verticals implement it for their own parse outputs (Java's
//! `ParsedJavaFile`, and a Kotlin equivalent later).
//!
//! The dynamic dispatch is one virtual call per *file*, not per graph
//! node — negligible next to parsing. This is about ownership and the
//! thread handoff, not runtime plugin extensibility: jobs are created and
//! consumed within a single index/update call.
//!
//! Invariants a job upholds so it can ride a rayon worker and cross the
//! thread boundary safely:
//!
//! - It is `Send` and fully owned. It holds no graph references, registry
//!   handles, subscriptions, borrowed source text, `NodeId`s, or protocol
//!   types — only the self-contained data integration needs. The `Send`
//!   supertrait is what lets `Box<dyn IntegrationJob<P>>` move from a
//!   parse worker to the graph thread.
//! - All graph and registry mutation happens inside
//!   [`integrate`](IntegrationJob::integrate), on the committing thread.
//!   Workers only build jobs.

use std::path::Path;

use crate::Interner;
use crate::graph::{Graph, NodeBehavior, NodeId};

/// One file's parsed output, ready to be committed into the graph.
///
/// `P` is the graph's payload union (the facade's `NodePayload`). The
/// bound `P: NodeBehavior` ties the job to the payload's lifecycle-hook
/// context ([`NodeBehavior::Ctx`] — the consumer's registries), which
/// [`integrate`](Self::integrate) threads through node registration.
///
/// Implemented by each vertical for its parse output and committed
/// through `Box<dyn IntegrationJob<P>>`, so one serial loop can drain a
/// batch of jobs from mixed languages.
pub trait IntegrationJob<P>: Send
where
    P: NodeBehavior,
{
    /// The source file this job was parsed from. The committer uses it to
    /// evict the file's prior nodes before integrating the new ones and
    /// to key any per-file bookkeeping.
    fn path(&self) -> &Path;

    /// Consume the job, inserting its nodes into `graph` and registering
    /// them against `ctx`, and return the inserted [`NodeId`]s in
    /// insertion order. Runs serially on the graph thread — the only
    /// point a job touches the engine.
    fn integrate(
        self: Box<Self>,
        graph: &mut Graph<P>,
        ctx: &P::Ctx,
        interner: &Interner,
    ) -> Vec<NodeId>;
}
