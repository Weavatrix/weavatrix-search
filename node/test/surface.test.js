'use strict'

// Exercises every exported member and every documented option key, so a
// broken or missing surface fails here rather than in a consumer.

const assert = require('node:assert/strict')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const test = require('node:test')
const search = require('..')

function fixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'weavatrix-search-surface-'))
  fs.mkdirSync(path.join(root, 'src'))
  fs.writeFileSync(path.join(root, 'src', 'a.ts'), 'const alpha = 1\nexport function run() { return alpha }\n')
  fs.writeFileSync(path.join(root, 'src', 'b.ts'), 'const alpha = 2\nconst beta = alpha\n')
  fs.writeFileSync(path.join(root, 'src', 'wide.txt'), `${'x'.repeat(4096)}alpha\n`)
  fs.writeFileSync(path.join(root, 'src', 'binary.bin'), Buffer.from([0x00, 0x61, 0x6c, 0x70, 0x68, 0x61]))
  fs.writeFileSync(path.join(root, 'src', 'utf16.txt'), Buffer.concat([
    Buffer.from([0xff, 0xfe]),
    Buffer.from('alpha\n', 'utf16le'),
  ]))
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

test('exports exactly the documented surface', () => {
  assert.deepEqual(
    Object.keys(search).sort(),
    ['Index', 'buildIndex', 'buildIndexSync', 'openIndex', 'search', 'searchSync'],
  )
  for (const name of ['buildIndex', 'buildIndexSync', 'openIndex', 'search', 'searchSync']) {
    assert.equal(typeof search[name], 'function', `${name} must be callable`)
  }
  assert.deepEqual(
    Object.getOwnPropertyNames(search.Index.prototype).sort(),
    ['applyEvents', 'buildReport', 'constructor', 'fileCount', 'rebuild', 'revision', 'save', 'search', 'searchSync', 'status'],
  )
  assert.throws(() => new search.Index({}), /use openIndex, buildIndex, or buildIndexSync/)
})

test('accepts one root or several and reports both', async () => {
  await withFixture(async (root) => {
    const one = await search.search(root, 'alpha')
    const many = await search.search([path.join(root, 'src'), root], 'alpha')
    assert.equal(one.roots.length, 1)
    assert.equal(many.roots.length, 2)
    assert.ok(many.occurrences > one.occurrences)
    assert.deepEqual([...new Set(many.matches.map((found) => found.rootIndex))].sort(), [0, 1])
  })
})

test('carries every documented search option', async () => {
  await withFixture(async (root) => {
    const report = await search.search(root, { regex: '(alpha)' }, {
      case: 'smart',
      beforeContext: 1,
      afterContext: 1,
      maxResults: 100,
      maxWarnings: 10,
      fileEvidence: 'all',
      maxFileEvidence: 100,
      maxFileBytes: 1024 * 1024,
      maxLineBytes: 8192,
      maxMultilineBytes: 1024 * 1024,
      replacement: 'beta',
      maxReplacementBytes: 4096,
      mode: 'line',
      resultMode: 'matches',
      encoding: 'auto',
      binary: 'skip',
      errorPolicy: 'continue',
      archives: {
        enabled: true,
        maxArchiveBytes: 1024 * 1024,
        maxEntryBytes: 1024 * 1024,
        maxExpandedBytes: 1024 * 1024,
        maxEntries: 100,
        maxDecoderMemoryBytes: 1024 * 1024,
      },
      scan: {
        extensions: ['ts', 'txt', 'bin'],
        overrideRules: ['!never-matched'],
        ignoreFiles: ['.gitignore'],
        skipHidden: true,
        parallelism: 2,
        maxEntries: 1000,
        maxTotalBytes: 8 * 1024 * 1024,
        timeoutMs: 60_000,
      },
    })
    assert.ok(report.occurrences > 0)
    assert.equal(report.matches[0].replacementPreview.includes('beta'), true)
    assert.ok(report.fileEvidence.length > 0)
    assert.equal(report.fileEvidenceTruncated, false)
    assert.equal(report.warningsDropped, 0)
    assert.equal(report.scan.roots[0].termination, null)
    assert.equal(report.scan.roots[0].portable, true)
    assert.ok(report.scan.roots[0].discovered >= report.scan.roots[0].completed)
  })
})

test('reports binary skips, UTF-16 decoding, and long-line warnings', async () => {
  await withFixture(async (root) => {
    const skipped = await search.search(root, 'alpha', { binary: 'skip' })
    assert.ok(skipped.warnings.some((warning) => warning.kind === 'binary'))

    const searched = await search.search(root, 'alpha', { binary: 'search' })
    assert.ok(searched.matches.some((found) => found.path.endsWith('binary.bin')))

    const utf16 = await search.search(root, 'alpha', { encoding: 'auto' })
    const decoded = utf16.matches.find((found) => found.path.endsWith('utf16.txt'))
    assert.equal(decoded.encoding, 'UTF-16LE')
    assert.equal(decoded.lossy, false)

    const clipped = await search.search(root, 'alpha', { maxLineBytes: 64 })
    assert.ok(clipped.warnings.some((warning) => warning.kind === 'line-too-long'))
  })
})

test('multiline mode spans line boundaries', async () => {
  await withFixture(async (root) => {
    const report = await search.search(root, { regex: 'alpha[\\s\\S]*beta' }, { mode: 'multiline' })
    const found = report.matches.find((match) => match.path.endsWith('b.ts'))
    assert.ok(found, 'multiline must cross the newline')
    assert.ok(found.endLineNumber > found.lineNumber)
  })
})

test('truncation and quiet mode stay bounded', async () => {
  await withFixture(async (root) => {
    const truncated = await search.search(root, 'alpha', { maxResults: 1 })
    assert.equal(truncated.matches.length, 1)
    assert.equal(truncated.truncated, true)

    const files = await search.search(root, 'alpha', { resultMode: 'files' })
    assert.equal(files.matches.length, 0)
    assert.ok(files.matchedFiles.length > 0)
  })
})

test('index handles save, reopen, status, rebuild, and events', async () => {
  await withFixture(async (root) => {
    const first = path.join(root, 'first.index')
    const second = path.join(root, 'second.index')
    const index = search.buildIndexSync(path.join(root, 'src'), { buildParallelism: 2, searchParallelism: 2 })
    assert.equal(index.save(first), index, 'save must be chainable')
    index.save(second)

    const reopened = search.openIndex(second, { maxEntries: 1000, maxContentBytes: 1 << 20, maxIndexBytes: 1 << 24, maxPathBytes: 4096 })
    assert.equal(reopened.revision, index.revision)
    assert.equal(reopened.buildReport(), undefined, 'an opened index has no build report')
    const status = reopened.status()
    assert.deepEqual(Object.keys(status).sort(), ['contentBytes', 'files', 'revision', 'roots'])
    assert.equal(status.files, reopened.fileCount)

    const rebuilt = reopened.rebuild()
    assert.equal(rebuilt.fullRebuild, true)
    assert.equal(rebuilt.changedScan.complete, true)
    assert.equal(rebuilt.files, reopened.fileCount)

    fs.rmSync(path.join(root, 'src', 'b.ts'))
    const removal = reopened.applyEvents(0, [{ path: path.join(root, 'src', 'b.ts'), kind: 'remove' }])
    assert.equal(removal.removed, 1)
    assert.equal((await reopened.search('beta')).occurrences, 0)

    const rescan = reopened.applyEvents(0, [{ path: path.join(root, 'src'), kind: 'rescan' }])
    assert.equal(rescan.fullRebuild, true)
  })
})

test('cancellation, root validation, and option typing are enforced', async () => {
  await withFixture(async (root) => {
    const controller = new AbortController()
    const running = search.search(root, 'alpha', { signal: controller.signal })
    controller.abort()
    await assert.rejects(running, { name: 'AbortError' })

    assert.throws(() => search.searchSync(root, { literal: '' }), /non-empty string/)
    assert.throws(() => search.searchSync(root, { any: [] }), /non-empty array/)
    assert.throws(() => search.searchSync(root, { literal: 'a', regex: 'b' }), /exactly one of/)
    assert.throws(() => search.searchSync(root, 42), /must be a string/)
    assert.throws(() => search.searchSync(['', root], 'alpha'), /non-empty string/)
    assert.throws(() => search.openIndex(path.join(root, 'missing.index')), { code: 'GenericFailure' })
    assert.throws(() => search.buildIndexSync(root, { scan: { unknown: 1 } }), /unknown/)
  })
})
