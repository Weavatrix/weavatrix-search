# weavatrix-search

Bounded, ignore-aware repository content search behind Weavatrix, exposed as a native library for Node.js and Bun. It is the Rust `weavatrix-search` core through Node-API—not a JavaScript rewrite, not a CLI wrapper, and not an MCP server.

## Install

```console
npm install weavatrix-search
# or
bun add weavatrix-search
```

```js
const { search, buildIndex, openIndex } = require('weavatrix-search')

const report = await search(process.cwd(), 'TODO', { case: 'smart', afterContext: 1 })
for (const found of report.matches) {
  console.log(`${found.path}:${found.lineNumber}: ${found.line}`)
}
console.log(report.occurrences, 'occurrences in', report.filesWithMatches, 'files')
```

Queries are a literal string, `{ regex }`, or `{ any: [...] }` for an ordered
multi-pattern pass. `search` runs outside the JavaScript event loop and accepts
an `AbortSignal`; `searchSync` is the blocking form.

## Persistent index

```js
const index = await buildIndex(process.cwd(), { path: '.weavatrix/search.index' })
console.log(index.buildReport().files, 'files at revision', index.revision)

const reopened = openIndex('.weavatrix/search.index')
const hits = await reopened.search({ regex: 'export function \\w+' })

// Feed watcher deltas instead of rescanning.
reopened.applyEvents(0, [{ path: '/repo/src/a.ts', kind: 'modify' }])
```

`applyEvents` accepts `create`, `modify`, `remove`, `renameFrom`, `renameTo`, `directory`, and `rescan`. Plans that can change selection promote themselves to a full rebuild and say so in `fullRebuild`.

## Native product boundary

One self-contained npm package supports Node.js 18+ and Bun 1.4+ and includes Windows, macOS, and glibc Linux bindings for x64 and arm64. It has no install script, performs no network download, creates no public platform-package names, and keeps traversal, decoding, matching, and index storage in Rust.

The surface covers literal/regex/multi-pattern queries, case policy, line and multiline modes, context lines, non-mutating replacement previews, count/files/quiet result modes, per-file evidence, archive member search, encoding policy, deterministic warnings, retention limits, cancellation, and persistent indexes with watcher updates.

## Measured

[`benchmark/RESULTS.md`](benchmark/RESULTS.md) compares an identical sorted match list against `fdir` plus `fs.readFileSync` plus per-line JavaScript testing. On a 15.9 MB corpus Weavatrix won a cold search by 3.15x on Node and 3.50x on Bun, and a repeated query through a persistent index by 212.57x and 209.58x.

Repository: [Weavatrix/weavatrix-search](https://github.com/Weavatrix/weavatrix-search) · Rust crate: [crates.io/crates/weavatrix-search](https://crates.io/crates/weavatrix-search) · License: [MIT](https://github.com/Weavatrix/weavatrix-search/blob/main/LICENSE)
