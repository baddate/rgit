# rgit UX Enhancements Design

## Summary

Four independent feature additions to bring rgit closer to cgit feature parity:
stats page, branch quick-switch dropdown, enhanced diff page with context control,
and log page branch graph visualization.

All features follow cgit's philosophy: pure server-side rendering, URL-parameter-driven,
minimal JavaScript (one-liner onchange redirects at most).

## Feature 1: Stats Page

### Route
`GET /:repo/stats`

### UI Layout
Four sections top to bottom:
1. **Summary bar** — total commits, total contributors, total files (latest HEAD)
2. **Contributor table** — author name, commit count, lines added/removed
3. **Activity chart** — monthly commit count as pure CSS bar chart (no JS library)
4. **Language breakdown** — file extension → count, percentage bar

### Data Collection
All data derived from RocksDB `commit_tree` traversal:

| Metric | Source |
|--------|--------|
| Total commits | `commit_tree.len()` |
| Contributor commit counts | Aggregate author from indexed commit data (already stored) |
| Contributor lines added/removed | Precomputed during indexer run from gix diff of each commit |
| Monthly activity | Group commits by `author.time` year-month (precomputed) |
| Language stats | Traverse latest HEAD tree, count by file extension (computed on request) |

### Caching
Stats are precomputed during `indexer::update_repository_reflog` — the indexer already iterates all new commits. Stats are persisted to RocksDB in a new column family or stored alongside the repository record. The stats handler reads precomputed data, never does on-demand traversal. Invalidation happens on each indexer run (incremental; only new commits trigger recomputation).

### Files
```
src/methods/repo/stats.rs     — handler + data aggregation
templates/repo/stats.html      — template
src/database/schema/stats.rs   — optional RocksDB stats storage
```

---

## Feature 2: Branch Quick-Switch Dropdown

### Placement
Top-right of repo nav bar, inside `extra_nav_links` block in `templates/repo/base.html`.

### Implementation
```html
<select onchange="location.href=this.value">
  <option value="?h=main" selected>🌿 main</option>
  <option value="?h=develop">develop</option>
  ...
</select>
```

- One `<select>` element, current branch gets `selected` attribute
- `onchange` sets `location.href` to `?h=<branch>` (relative URL preserves current page path + other params)
- Branch list obtained from `repository.get().heads(&db)` — already used in summary handler

### Data Flow
Each page handler queries `heads` from RocksDB (fast prefix scan, <1ms) and passes `Vec<(String, bool)>` (name, is_current) to the template. No middleware needed.

### Affected Files
```
templates/repo/base.html  — add <select> in extra_nav_links
src/methods/repo/log.rs   — add heads to View struct
src/methods/repo/tree.rs  — add heads to View struct
src/methods/repo/commit.rs — add heads to View struct
src/methods/repo/diff.rs  — add heads to View struct
src/methods/repo/refs.rs  — add heads to View struct
src/methods/repo/summary.rs — heads already present
```

---

## Feature 3: Enhanced Diff Page

### Context Control
- URL parameter: `?context=N` (default: 3)
- Floating sticky toolbar at top of diff content area (CSS `position: sticky; top: 0`)
- `<select>` dropdown: 3, 5, 10, 25, full
- `full` maps to `u32::MAX` context (show entire file surrounding each hunk)
- `context` value is passed through to gix `blob-diff` via `gix::diff::blob::Platform::context()`
- On change: `onchange` rewrites the `context` query param in current URL

### Enhanced Diff Format
Replace raw `<pre>{{ diff_text }}</pre>` with structured rendering:

**File header** (per changed file):
```
📄 src/main.c  +12 −3
```

**Row format** (with line numbers):
```
  .   1  | int main() {
  -    2  |     printf("old");
  +      |     printf("new");
  .   3  |     return 0;
```

### Rust-Side Changes
Parse gix `blob-diff` output into `Vec<DiffFile>`:
```rust
struct DiffFile {
    path: String,        // file path from diff header
    hunks: Vec<DiffHunk>,
    lines_added: usize,
    lines_removed: usize,
}

struct DiffHunk {
    old_start: usize,
    new_start: usize,
    lines: Vec<DiffLine>,  // Context | Added | Removed
}
```

### Styling
- Dark/light mode compatible via existing CSS variables
- Added/modified/removed lines: distinct background colors
- Line numbers: muted color, right-aligned

### Files
```
src/methods/repo/diff.rs        — context param, structured diff parsing
templates/repo/diff.html         — enhanced template with file headers + line numbers
statics/sass/diff.scss           — updated diff styles
```

---

## Feature 4: Log Page Branch Graph

### Route
`GET /:repo/log?h=<branch>&ofs=<offset>&graph=<style>`

### Configuration
```ini
[rgit]
    log-graph = "ascii"   # ascii | unicode | table
```
URL param `?graph=` overrides config. Default: `ascii`.

### Algorithm: DAG Layout

1. **Collect**: For each commit on current page, collect parent OIDs
2. **Assign lanes**: Map commits to column positions based on branch topology
3. **Render edges**: Generate connection characters per lane/edge relationship

Data model:
```rust
struct GraphCommit {
    commit: YokedCommit,
    lane: usize,            // primary column
    lanes: Vec<GraphCell>,  // columns in this row
}

enum GraphCell {
    Empty,
    Node,         // ● or *
    Line,         // │ or |
    Branch,       // ├── or |\
    Merge,        // ┌── or |\
    Continuation, // │ up/down across page boundary
}
```

### Three Render Modes

| Mode | Node | Line | Branch | Merge |
|------|------|------|--------|-------|
| ASCII | `*` | `\|` | `\|\` | `\|/` |
| Unicode | `●` | `│` | `├──` | `┌──` |
| Table | Same as Unicode but rendered in `<td>` columns |

All three share the same layout algorithm. Only character selection differs.
Template macro `branch_graph.html` handles rendering based on mode.

### Edge Cases
- **Linear history**: Single lane, no branch/merge symbols
- **Octopus merge**: Merge commit with >2 parents — render `|` for extra parents
- **Page boundary**: Cross-page edges show `│` continuation marker
- **First page**: No continuation markers at top

### Files
```
src/methods/repo/log.rs            — graph computation, graph param
src/git.rs                          — optional: graph layout algorithm
templates/repo/log.html             — updated template
templates/repo/macros/branch_graph.html — new: graph rendering macro
statics/sass/style.scss             — graph column styles
```

---

## Cross-Cutting Concerns

### CSS
All new styles use existing SCSS variables (`_colours.scss`) for dark/light mode consistency.
No new external dependencies.

### No New JS Dependencies
The only JavaScript is inline `onchange` handlers on `<select>` elements.
Total JS footprint: <10 lines.

### Performance Targets
- Stats page: <500ms for repos up to 50K commits (with RocksDB caching)
- Branch dropdown: no measurable overhead (RocksDB prefix scan ~microseconds)
- Diff: no regression vs current; enhanced format adds minimal string processing
- Log graph: <50ms additional overhead for 100-commit page

### Testing Strategy
- Unit tests for graph layout algorithm (linear, branch, merge, octopus merge)
- Unit tests for diff parsing (single file, multi-file, binary file skip)
- Snapshot tests for stats aggregation on known test repo
