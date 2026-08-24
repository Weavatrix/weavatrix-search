# weavatrix-search

Bounded, ignore-aware repository content search, written in Rust and exposed to
Node.js and Bun through Node-API.

It walks a repository the way a code tool should — respecting ignore files,
detecting binaries, decoding encodings, staying inside declared memory limits —
and returns matches with exact line and byte evidence. It spawns no process and
wraps no CLI.

```console
npm install weavatrix-search
# or
bun add weavatrix-search
```

```js
const { search } = require('weavatrix-search')

const report = await search(process.cwd(), 'TODO', { case: 'smart', afterContext: 1 })
for (const found of report.matches) {
  console.log(`${found.path}:${found.lineNumber}: ${found.line}`)
}
report.occurrences      // total matches
report.filesWithMatches // files that contained at least one
```

---

## Two modes

**Filesystem search** walks and matches in one pass. Use it for a one-off
query.

**A persistent index** stores an exact, revisioned content snapshot on disk.
Use it when the same repository is queried repeatedly — an editor, an agent, a
language server — because a repeat query then costs no filesystem read at all.
Watcher events update the snapshot in place instead of rescanning.

---

## API

### `search(roots, query, options?) → Promise<SearchReport>`

Runs outside the JavaScript event loop on a worker thread.

### `searchSync(roots, query, options?) → SearchReport`

The blocking form. Same arguments, same report.

| Parameter | Type | Notes |
| --- | --- | --- |
| `roots` | `string \| string[]` | One or several independent repository roots. Every match carries the `rootIndex` it came from, in insertion order. |
| `query` | `Query` | See below. |
| `options` | `SearchOptions` | See below. |

### `Query`

| Form | Meaning |
| --- | --- |
| `'text'` | Shorthand for `{ literal: 'text' }`. |
| `{ literal: string }` | Matches the text literally. |
| `{ regex: string }` | Rust regular-expression syntax. |
| `{ any: Query[] }` | The ordered union of several queries in **one** content pass. Order resolves alternatives that begin at the same byte, like repeated `-e` in grep. Each match reports which pattern hit through `spans[].patternIndex`. |

### `SearchOptions`

| Option | Type | Default | Effect |
| --- | --- | --- | --- |
| `case` | `'sensitive' \| 'insensitive' \| 'smart'` | `'sensitive'` | `smart` ignores case unless the query contains an uppercase character. |
| `beforeContext` | `number` | `0` | Lines retained before each match. |
| `afterContext` | `number` | `0` | Lines retained after each match. |
| `maxResults` | `number` | `10000` | Match records retained in memory. Reaching it sets `truncated` and leaves the counters complete. |
| `maxWarnings` | `number` | `1000` | Warnings retained. The overflow count lands in `warningsDropped`. |
| `fileEvidence` | `'none' \| 'matched' \| 'all'` | `'none'` | Retains per-file text metrics computed during the same pass as matching. |
| `maxFileEvidence` | `number` | `100000` | Metric records retained; overflow sets `fileEvidenceTruncated`. |
| `maxFileBytes` | `number` | `33554432` | Largest ordinary source file accepted. |
| `maxLineBytes` | `number` | `8388608` | Largest logical line retained; longer lines produce a `line-too-long` warning. |
| `maxMultilineBytes` | `number` | `8388608` | Source bytes buffered in multiline mode. |
| `replacement` | `string` | — | Renders a **non-mutating** preview per match into `replacementPreview`. `$0`, numbered groups, named groups, and `$$` follow `regex-automata` syntax. Nothing on disk is touched. |
| `maxReplacementBytes` | `number` | `8388608` | Byte limit for one rendered preview. |
| `mode` | `'line' \| 'multiline'` | `'line'` | `multiline` lets a match cross line boundaries under the byte limit above. |
| `resultMode` | `'matches' \| 'count' \| 'files' \| 'quiet'` | `'matches'` | `count` keeps aggregate counters and per-file summaries; `files` keeps bounded per-file summaries; `quiet` stops every worker after the first match. |
| `encoding` | `string` | `'auto'` | `auto` detects UTF-8/UTF-16 BOMs and otherwise assumes UTF-8. Also accepts `utf8`, `utf16le`, `utf16be`, or any `encoding_rs` label such as `windows-1252`. |
| `binary` | `'skip' \| 'search'` | `'skip'` | `skip` records a `binary` warning for input with a NUL byte near the start; `search` decodes and searches it anyway. |
| `errorPolicy` | `'continue' \| 'abort'` | `'continue'` | Whether a per-file failure stops the whole operation. |
| `archives` | `ArchiveOptions` | enabled | See below. |
| `scan` | `ScanOptions` | tuned per root | Adjustments layered on the scanner profile recommended for these roots. |
| `signal` | `AbortSignal` | — | Cancels an in-flight search cooperatively; the promise rejects with `AbortError`. |

Unknown option keys are rejected with `InvalidArg` rather than ignored.

#### `ArchiveOptions`

Archive members are searched in place and reported as `archive!member` virtual
paths with `archive: true`.

| Option | Default | Effect |
| --- | --- | --- |
| `enabled` | `true` | Archive recognition and member search. |
| `maxArchiveBytes` | `33554432` | Compressed bytes accepted for one archive. |
| `maxEntryBytes` | `16777216` | Expanded bytes accepted for one member. |
| `maxExpandedBytes` | `134217728` | Cumulative expanded bytes for one archive. |
| `maxEntries` | `10000` | Members visited in one archive. |
| `maxDecoderMemoryBytes` | `67108864` | Decoder scratch memory where the format exposes a bound. |

Formats: ZIP and TAR containers; GZIP, BZip2, Zstandard, LZ4 frame, raw LZMA,
XZ, and Brotli streams.

#### `ScanOptions`

| Option | Effect |
| --- | --- |
| `extensions` | Restricts selection to these extensions. |
| `overrideRules` | Gitignore-syntax rules applied above discovered ignore files. |
| `ignoreFiles` | Which ignore filenames to honour. |
| `skipHidden` | Whether dotfiles and dot-directories are skipped. |
| `parallelism` | Content workers. |
| `maxEntries`, `maxTotalBytes` | Hard scan bounds; hitting one sets `termination`. |
| `timeoutMs` | Wall-clock bound. A timeout returns a **partial report** with `scan.roots[].termination === 'timeout'` rather than throwing, so "take what you got in 2 seconds" needs no `AbortSignal`. |

### `buildIndex(roots, options?) → Promise<Index>`

Builds a complete snapshot off the event loop.

### `buildIndexSync(roots, options?) → Index`

The blocking form.

| Option | Type | Effect |
| --- | --- | --- |
| `path` | `string` | Saves the snapshot atomically as part of the build and excludes the storage file from its own scan. |
| `maxEntries`, `maxContentBytes`, `maxIndexBytes`, `maxPathBytes` | `number` | Hard resource bounds. |
| `buildParallelism`, `searchParallelism` | `number` | Worker counts. |
| `scan` | `ScanOptions` | As above. |
| `signal` | `AbortSignal` | Cancels the build. |

### `openIndex(path, options?) → Index`

Opens a saved snapshot without rescanning anything.

### `class Index`

| Member | Returns | Notes |
| --- | --- | --- |
| `fileCount` | `number` | Selected files in the snapshot. |
| `revision` | `string` | Deterministic hash of roots, paths, and content. Two identical repositories produce the same revision. |
| `status()` | `IndexStatus` | `{ roots, files, contentBytes, revision }` — cheap enough for a health endpoint. |
| `buildReport()` | `IndexBuildReport \| undefined` | Present only on a handle that came from a build. |
| `save(path)` | `this` | Atomic write. |
| `search(query, options?)` | `Promise<SearchReport>` | Off the event loop. `backend` is `'persistent-index'` and `index` carries the prefilter evidence. |
| `searchSync(query, options?)` | `SearchReport` | |
| `applyEvents(rootIndex, events, options?)` | `IndexUpdateReport` | Applies watcher deltas without traversal. |
| `rebuild(options?)` | `IndexUpdateReport` | Rescans every indexed root and replaces the snapshot in place. |

`events` is an array of `{ path, kind }` where `kind` is `create`, `modify`,
`remove`, `renameFrom`, `renameTo`, `directory`, or `rescan`. Plans that could
change *selection* — a directory event, an ignore-file change, an explicit
rescan — promote themselves to a full rebuild and report `fullRebuild: true`
rather than silently drifting.

```js
const fs = require('node:fs')
const index = await buildIndex(root, { path: '.weavatrix/search.index' })

fs.watch(root, { recursive: true }, (event, name) => {
  index.applyEvents(0, [{ path: `${root}/${name}`, kind: event === 'rename' ? 'create' : 'modify' }])
})
```

---

## `SearchReport`

| Field | Type | Meaning |
| --- | --- | --- |
| `backend` | `'filesystem' \| 'persistent-index' \| 'live-index'` | Which content source served this query. |
| `index` | `IndexSearchEvidence \| null` | `{ revision, indexedFiles, candidateFiles, prefiltered }`. The trigram prefilter only ever *rejects* impossible candidates; every surviving candidate is verified by the normal engine. |
| `roots` | `string[]` | In insertion order. |
| `resultMode` | `string` | The mode this report was built with. |
| `matches` | `SearchMatch[]` | Sorted by root, path, line, then first span. Empty in `count`, `files`, and `quiet` modes. |
| `matchingLines`, `occurrences`, `filesWithMatches` | `number` | Aggregate counters; complete even when `matches` was truncated. |
| `filesSearched`, `bytesSearched` | `number` | What the content pipeline actually processed. |
| `truncated` | `boolean` | Whether `maxResults` dropped records. |
| `warnings` | `SearchWarning[]` | `{ path, kind, message }` with `kind` in `binary`, `encoding`, `line-too-long`, `archive`, `limit`. |
| `matchedFiles` | `MatchedFile[]` | Per-file summaries for `count` and `files` modes. |
| `fileEvidence` | `SourceFileEvidence[]` | Per-file metrics when requested. |
| `fileEvidenceTruncated`, `warningsDropped` | | Retention overflow. |
| `scan` | `ScanReport` | `{ complete, cancelled, roots[] }` with per-root `discovered`, `completed`, `bytesEmitted`, `termination`, `portable`. |

### `SearchMatch`

| Field | Meaning |
| --- | --- |
| `rootIndex`, `path` | Which root, and the normalized path (or `archive!member`). |
| `lineNumber`, `endLineNumber` | One-based; they differ only in multiline mode. |
| `decodedByteOffset` | Line start in decoded UTF-8 bytes. |
| `sourceByteOffset` | Exact line start in *source* bytes when a lossless mapping exists, otherwise `null`. |
| `line` | The decoded matching block. Multiline mode preserves embedded terminators. |
| `replacementPreview` | The rendered preview, or `null`. |
| `spans` | Non-overlapping `{ patternIndex, start, end }` ranges inside `line`. |
| `before`, `after` | Context lines in source order. |
| `encoding`, `lossy` | The effective decoder, and whether malformed input needed replacement characters. |
| `archive` | Whether `path` addresses a virtual archive member. |

---

## Errors

| `code` | Cause |
| --- | --- |
| `InvalidArg` | Unknown option key or enum value, malformed query, empty root list. |
| `GenericFailure` | Invalid regular expression, unreadable root, index load failure. |
| — (`AbortError`) | The supplied `AbortSignal` fired. |

---

## What ships

| | |
| --- | --- |
| Runtimes | Node.js 18+ (Node-API 8), Bun 1.4+ |
| Platforms | Windows x64/arm64, macOS x64/arm64, glibc Linux x64/arm64 |
| Install script | none |
| Network at install | none |
| Runtime dependencies | none |
| Platform packages | none — all six bindings are in this one tarball |

---

## Measured

[`benchmark/RESULTS.md`](benchmark/RESULTS.md) is generated from the
[weavatrix-benchmarks](https://github.com/Weavatrix/weavatrix-benchmarks)
harness, which forces both sides to return the identical sorted match list
before either is timed.

The competitor is what a Node or Bun project would otherwise write: `fdir` plus
`fs.readFileSync` plus a per-line test. **This is not a ripgrep comparison** —
ripgrep is not an npm library.

Medians of three independent runs over a 15.9 MB corpus, each in a fresh
process:

| Contract | Node 24 | Bun 1.3 |
| --- | ---: | ---: |
| Cold repository search | **4.66x** (4.59–4.82) | **4.75x** (4.02–5.00) |
| Repeat query through a persistent index | **253.8x** (245–291) | **233.1x** (221–301) |

The cold row understates the difference in work done: Weavatrix also applies
ignore rules, binary detection, and encoding handling that the baseline skips.
The second row is the case an editor or agent actually hits — the baseline
re-reads all 15.9 MB every time.

---

Repository: [Weavatrix/weavatrix-search](https://github.com/Weavatrix/weavatrix-search) ·
Rust crate: [crates.io/crates/weavatrix-search](https://crates.io/crates/weavatrix-search) ·
License: [MIT](https://github.com/Weavatrix/weavatrix-search/blob/main/LICENSE)
