# Node.js and Bun benchmark snapshot

Measured on 2026-08-24 on Windows x64 over a generated corpus of 2,000
TypeScript files, 200 lines each, 15,920,400 bytes total, containing 80
matching lines. Both sides must return the identical sorted list of
`{ path, lineNumber, line }` before either is timed. Values are medians of
seven measured rounds after two warm-up rounds, with execution order
alternating per round.

The competitor is the idiomatic JavaScript way to search a repository:
`fdir` 6.5.0 crawls the tree, `fs.readFileSync` reads each file, and each line
is tested in JavaScript.

| Contract | Runtime | Weavatrix | fdir + fs baseline | Result |
| --- | --- | ---: | ---: | ---: |
| Cold repository search | Node 24.15.0 | 109.682 ms | 345.568 ms | Weavatrix 3.15x faster |
| Cold repository search | Bun 1.3.14 | 120.056 ms | 420.062 ms | Weavatrix 3.50x faster |
| Repeat query, persistent index | Node 24.15.0 | 1.824 ms | 387.730 ms | Weavatrix 212.57x faster |
| Repeat query, persistent index | Bun 1.3.14 | 1.846 ms | 386.882 ms | Weavatrix 209.58x faster |

The two contracts answer different questions. The first is a single cold
search, where Weavatrix wins on the content pipeline alone while also applying
ignore rules, binary detection, and encoding handling that the baseline does
not. The second is the case an editor or agent actually hits — the same corpus
queried repeatedly — where a persistent index removes the filesystem read
entirely and the baseline has to re-read all 15.9 MB every time.

This is not a comparison against `ripgrep`. `ripgrep` is not an npm library,
and the Rust-side comparison against it lives in `benches/compare_ripgrep.rs`
in this repository. The row above measures what a Node or Bun consumer would
otherwise write.

Reproduce from `node/`:

```console
npm ci
npm run build
npm run bench
bun run benchmark/node-baseline.mjs
```

`WV_SEARCH_FILES`, `WV_SEARCH_LINES`, and `WV_SEARCH_ROUNDS` change the
corpus and round count. Filesystem cache, antivirus, storage, and corpus shape
can materially change these timings. Treat them as a reproducible snapshot,
not a universal result.
