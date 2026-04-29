# Backlog

Local-first task tracking for beans. Each item is a markdown file with YAML
frontmatter, designed to be greppable, reviewable in PRs, and migratable to
GitHub Issues when the project outgrows it.

## Why local-first

Tracking work in markdown files keeps planning in the same place as the code
and the ADRs. New contributors see the backlog when they clone the repo. PRs
that finish a task can edit the backlog item in the same diff. There is no
external system to keep in sync, no auth wall, no mid-air-collision when two
people edit the same item — git handles all of that.

The trade-off: this scales until roughly five to ten active contributors. Past
that, the lack of assignees, comments, and notifications starts to hurt, and
GitHub Issues is the next stop. The frontmatter format below is deliberately
shaped to migrate cleanly: `status` becomes the issue state, `area` and
`priority` become labels, the body becomes the issue description.

## File naming

`NNN-kebab-case-slug.md` where `NNN` is a 3-digit sequential number.

- Numbering is monotonic. When adding a new item, use the next free number.
- Never reuse numbers — even for items that were dropped or completed.
- The slug is short and descriptive: `019-kotlin-parser-skeleton.md`, not
  `019-kotlin.md` or `019-add-the-kotlin-language-parser-skeleton.md`.

## Frontmatter

```yaml
---
status: pending | in-progress | completed | dropped
area: core | java | kotlin | scala | groovy | clojure | graph | lsp | testing | docs | parser
priority: low | medium | high
---
```

- `status`: lifecycle state. `pending` is the default for new items.
  `in-progress` means someone is actively working on it. `completed` is
  reserved for items that have shipped. `dropped` is for items that were
  considered and deliberately rejected — keep them with a short note in the
  body explaining why.
- `area`: rough subsystem. Used for filtering. If something spans two areas,
  pick the dominant one.
- `priority`: how soon this should land relative to other work. Use `high`
  sparingly.

## Body structure

Three sections, each terse:

```markdown
# <Short imperative title>

## Description

What needs to be done. One paragraph or short list of bullet points.

## Context

Why this matters, what depends on it, what it depends on. Reference ADRs
by number (e.g., "see ADR-0014") or other backlog items by file name.

## Acceptance criteria

How we know it is done. Bullet list, each item testable.
```

Items should be 30-80 lines total. If an item grows beyond that, it probably
contains multiple tasks and should be split.

## Querying

Use `rg` against the backlog directory. The frontmatter is plain YAML, so
field-based queries work:

```sh
# All pending items
rg --files-with-matches -B0 'status: pending' backlog/

# All graph items, regardless of status
rg --files-with-matches 'area: graph' backlog/

# All high-priority items
rg --files-with-matches 'priority: high' backlog/

# Items mentioning a specific ADR
rg 'ADR-0009' backlog/

# Count by status
rg -h 'status:' backlog/ | sort | uniq -c
```

For more complex queries, use `fd` to list files and pipe to `rg`:

```sh
# Pending high-priority items in the graph area
fd . backlog -e md | xargs rg -l 'status: pending' \
  | xargs rg -l 'priority: high' \
  | xargs rg -l 'area: graph'
```

## Lifecycle

1. Create a new file with the next number and `status: pending`.
2. When you start work, flip to `status: in-progress`. Optionally add a brief
   note at the top of the body about who is working on it and on what branch.
3. When the work merges, flip to `status: completed`. Leave the file in place
   as historical record. Do not delete completed items.
4. If the item is no longer worth doing, flip to `status: dropped` and add a
   short note in the body explaining the rationale. Future readers should be
   able to understand why it was dropped without external context.

## When to migrate to GitHub Issues

Migrate when:

- More than a handful of people are picking up tasks concurrently and
  conflicts on the file-naming sequence become annoying.
- Cross-linking between tasks, PRs, and external bug reports becomes
  load-bearing (e.g., users filing issues that need triage).
- Notifications and assignment tracking matter more than diff history.

The migration is mechanical: each pending or in-progress backlog file becomes
one issue, with frontmatter mapped to labels and state. Completed and dropped
items can stay in the repo as historical record, or be archived into a single
appendix file.
