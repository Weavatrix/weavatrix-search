// Output-equivalent comparison against the idiomatic JavaScript way to search
// a repository: crawl with fdir, read every file, and test each line.
//
// Both sides must produce the identical sorted match list before either is
// timed. The second contract repeats the same query against an unchanged
// corpus, which is where the persistent index replaces filesystem reads.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { performance } from 'node:perf_hooks'
import { createRequire } from 'node:module'
import { fdir } from 'fdir'

const require = createRequire(import.meta.url)
const { searchSync, buildIndexSync } = require('../lib/index.js')

const files = Number(process.env.WV_SEARCH_FILES ?? 2_000)
const lines = Number(process.env.WV_SEARCH_LINES ?? 200)
const rounds = Number(process.env.WV_SEARCH_ROUNDS ?? 7)
const needle = 'weavatrixNeedle'
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'weavatrix-search-bench-'))

function median(samples) {
  const sorted = [...samples].sort((left, right) => left - right)
  return Number(sorted[Math.floor(sorted.length / 2)].toFixed(3))
}

function time(operation) {
  const start = performance.now()
  operation()
  return performance.now() - start
}

function measurePair(left, right) {
  const leftSamples = []
  const rightSamples = []
  for (let round = 0; round < rounds + 2; round += 1) {
    let leftElapsed
    let rightElapsed
    if (round % 2 === 0) {
      leftElapsed = time(left)
      rightElapsed = time(right)
    } else {
      rightElapsed = time(right)
      leftElapsed = time(left)
    }
    if (round >= 2) {
      leftSamples.push(leftElapsed)
      rightSamples.push(rightElapsed)
    }
  }
  return [median(leftSamples), median(rightSamples)]
}

function order(matches) {
  return matches.sort((left, right) =>
    left.path.localeCompare(right.path) || left.lineNumber - right.lineNumber)
}

function weavatrix() {
  return order(searchSync(root, needle).matches
    .map((found) => ({ path: found.path, lineNumber: found.lineNumber, line: found.line })))
}

function baseline() {
  const crawled = new fdir().withRelativePaths().crawl(root).sync()
  const matches = []
  for (const relative of crawled) {
    const normalized = relative.replaceAll('\\', '/')
    const text = fs.readFileSync(path.join(root, relative), 'utf8')
    const split = text.split('\n')
    for (let index = 0; index < split.length; index += 1) {
      if (split[index].includes(needle)) {
        matches.push({ path: normalized, lineNumber: index + 1, line: split[index] })
      }
    }
  }
  return order(matches)
}

try {
  const directories = Math.ceil(files / 100)
  for (let directory = 0; directory < directories; directory += 1) {
    const target = path.join(root, `pkg${directory}`)
    fs.mkdirSync(target)
    const count = Math.min(100, files - directory * 100)
    for (let index = 0; index < count; index += 1) {
      const body = Array.from({ length: lines }, (_, line) =>
        line === 7 && index % 25 === 0
          ? `export const value${line} = ${needle}(${index})`
          : `export const value${line} = compute(${index}, ${line})`)
      fs.writeFileSync(path.join(target, `module${index}.ts`), `${body.join('\n')}\n`)
    }
  }

  const expected = weavatrix()
  if (JSON.stringify(expected) !== JSON.stringify(baseline())) {
    throw new Error('match parity failed')
  }
  if (expected.length === 0) {
    throw new Error('benchmark corpus produced no matches')
  }

  const index = buildIndexSync(root)
  const indexed = () => order(index.searchSync(needle).matches
    .map((found) => ({ path: found.path, lineNumber: found.lineNumber, line: found.line })))
  if (JSON.stringify(indexed()) !== JSON.stringify(expected)) {
    throw new Error('index parity failed')
  }

  const [weavatrixMs, baselineMs] = measurePair(weavatrix, baseline)
  const [indexedMs, repeatBaselineMs] = measurePair(indexed, baseline)
  const bytes = fs.readdirSync(root)
    .flatMap((directory) => fs.readdirSync(path.join(root, directory))
      .map((file) => fs.statSync(path.join(root, directory, file)).size))
    .reduce((total, size) => total + size, 0)

  console.log(JSON.stringify({
    runtime: process.versions.bun ? `bun ${process.versions.bun}` : `node ${process.version}`,
    files,
    lines,
    corpusBytes: bytes,
    matches: expected.length,
    rounds,
    results: [
      {
        contract: 'cold repository search: identical sorted path/line/text match list',
        weavatrixMs,
        baselineMs,
        ratio: Number((baselineMs / weavatrixMs).toFixed(2)),
      },
      {
        contract: 'repeat query over an unchanged corpus through a persistent index',
        weavatrixMs: indexedMs,
        baselineMs: repeatBaselineMs,
        ratio: Number((repeatBaselineMs / indexedMs).toFixed(2)),
      },
    ],
  }, null, 2))
} finally {
  fs.rmSync(root, { recursive: true, force: true })
}
