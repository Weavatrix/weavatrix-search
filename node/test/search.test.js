'use strict'

const assert = require('node:assert/strict')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const test = require('node:test')
const { buildIndex, buildIndexSync, openIndex, search, searchSync } = require('..')

function fixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'weavatrix-search-test-'))
  fs.mkdirSync(path.join(root, 'src'))
  fs.mkdirSync(path.join(root, 'build'))
  fs.writeFileSync(path.join(root, '.gitignore'), 'build/\n')
  fs.writeFileSync(path.join(root, 'src', 'a.ts'), 'const alpha = 1\nexport function run() { return alpha }\n')
  fs.writeFileSync(path.join(root, 'src', 'b.ts'), 'import { run } from "./a"\nrun()\nrun()\n')
  fs.writeFileSync(path.join(root, 'build', 'a.ts'), 'const alpha = 999\n')
  return root
}

async function withFixture(body) {
  const root = fixture()
  try {
    return await body(root)
  } finally {
    fs.rmSync(root, { recursive: true, force: true })
  }
}

test('searches a repository and reports deterministic evidence', async () => {
  await withFixture(async (root) => {
    const report = await search(root, 'alpha')
    assert.equal(report.backend, 'filesystem')
    assert.equal(report.resultMode, 'matches')
    assert.deepEqual(report.matches.map((found) => found.path), ['src/a.ts', 'src/a.ts'])
    assert.equal(report.occurrences, 2)
    assert.equal(report.matchingLines, 2)
    assert.equal(report.filesWithMatches, 1)
    assert.deepEqual(report.matches[0].spans, [{ patternIndex: 0, start: 6, end: 11 }])
    assert.equal(report.scan.complete, true)
    assert.equal(report.scan.cancelled, false)
    assert.deepEqual(searchSync(root, 'alpha').matches, report.matches)
  })
})

test('honors ignore files, case policy, regex, and multi-pattern queries', async () => {
  await withFixture(async (root) => {
    assert.equal((await search(root, 'ALPHA')).occurrences, 0)
    assert.equal((await search(root, 'ALPHA', { case: 'insensitive' })).occurrences, 2)
    assert.equal((await search(root, { regex: 'run\\(\\)' })).occurrences, 3)
    const both = await search(root, { any: [{ literal: 'alpha' }, { regex: 'import' }] })
    assert.equal(both.occurrences, 3)
    assert.equal(both.matches.at(-1).spans[0].patternIndex, 1)
    assert.ok(both.matches.every((found) => !found.path.startsWith('build/')))
  })
})

test('supports counting, file, and quiet result modes with context and previews', async () => {
  await withFixture(async (root) => {
    const counted = await search(root, 'run', { resultMode: 'count' })
    assert.equal(counted.matches.length, 0)
    assert.equal(counted.occurrences, 4)
    assert.deepEqual(counted.matchedFiles.map((file) => file.path), ['src/a.ts', 'src/b.ts'])

    const quiet = await search(root, 'run', { resultMode: 'quiet' })
    assert.equal(quiet.filesWithMatches, 1)

    const context = await search(root, 'export', { beforeContext: 1, afterContext: 1 })
    assert.deepEqual(context.matches[0].before.map((line) => line.text), ['const alpha = 1'])

    const preview = await search(root, { regex: '(alpha)' }, { replacement: 'beta' })
    assert.equal(preview.matches[0].replacementPreview, 'const beta = 1')
  })
})

test('builds, saves, reopens, and updates a persistent index', async () => {
  await withFixture(async (root) => {
    const indexPath = path.join(root, 'search.index')
    const built = await buildIndex(root, { path: indexPath })
    assert.equal(built.fileCount, 2)
    assert.equal(built.buildReport().files, 2)
    assert.equal(built.buildReport().revision, built.revision)
    assert.equal(built.status().files, 2)

    const opened = openIndex(indexPath)
    assert.equal(opened.revision, built.revision)
    const report = await opened.search('alpha')
    assert.equal(report.backend, 'persistent-index')
    assert.equal(report.occurrences, 2)
    assert.equal(report.index.indexedFiles, 2)
    assert.deepEqual(opened.searchSync('alpha').matches, report.matches)

    fs.writeFileSync(path.join(root, 'src', 'c.ts'), 'const alpha = 3\n')
    const update = opened.applyEvents(0, [{ path: path.join(root, 'src', 'c.ts'), kind: 'create' }])
    assert.equal(update.added, 1)
    assert.equal(update.fullRebuild, false)
    assert.notEqual(update.revision, built.revision)
    assert.equal((await opened.search('alpha')).occurrences, 3)

    const rebuilt = opened.rebuild()
    assert.equal(rebuilt.fullRebuild, true)
    assert.equal(rebuilt.files, 3)

    const sync = buildIndexSync(path.join(root, 'src'))
    assert.equal(sync.fileCount, 3)
    assert.equal(sync.buildReport().files, 3)
    assert.equal(sync.buildReport().scan.complete, true)
  })
})

test('cancels an in-flight search through an AbortSignal', async () => {
  await withFixture(async (root) => {
    const controller = new AbortController()
    controller.abort()
    await assert.rejects(search(root, 'alpha', { signal: controller.signal }), (error) => {
      assert.equal(error.name, 'AbortError')
      return true
    })
    assert.throws(() => searchSync(root, 'alpha', { signal: controller.signal }), { name: 'AbortError' })
  })
})

test('rejects unusable queries and unknown options', async () => {
  await withFixture(async (root) => {
    await assert.rejects(search(root, 'x', { resultMode: 'everything' }), /unsupported resultMode/)
    await assert.rejects(search(root, 'x', { nonsense: true }), /nonsense/)
    assert.throws(() => searchSync(root, {}), /exactly one of literal, regex, or any/)
    assert.throws(() => searchSync([], 'x'), /at least one search root/)
    await assert.rejects(search(root, { regex: '(' }), /invalid regular expression/)
  })
})
