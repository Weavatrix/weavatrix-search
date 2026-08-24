'use strict'

const native = require('../index.js')

const QUERY_KEYS = ['literal', 'regex', 'any']

function encodeRoots(roots) {
  const list = Array.isArray(roots) ? roots : [roots]
  if (list.length === 0) {
    throw new TypeError('at least one search root is required')
  }
  for (const root of list) {
    if (typeof root !== 'string' || root.length === 0) {
      throw new TypeError('every search root must be a non-empty string')
    }
  }
  return JSON.stringify(list)
}

function normalizeQuery(query) {
  if (typeof query === 'string') {
    return { literal: query }
  }
  if (query == null || typeof query !== 'object') {
    throw new TypeError('a query must be a string, { literal }, { regex }, or { any }')
  }
  const keys = Object.keys(query).filter((key) => QUERY_KEYS.includes(key))
  if (keys.length !== 1 || Object.keys(query).length !== 1) {
    throw new TypeError('a query object must carry exactly one of literal, regex, or any')
  }
  if (keys[0] === 'any') {
    if (!Array.isArray(query.any) || query.any.length === 0) {
      throw new TypeError('{ any } must be a non-empty array of queries')
    }
    return { any: query.any.map(normalizeQuery) }
  }
  if (typeof query[keys[0]] !== 'string' || query[keys[0]].length === 0) {
    throw new TypeError(`{ ${keys[0]} } must be a non-empty string`)
  }
  return { [keys[0]]: query[keys[0]] }
}

function encodeOptions(options, dropped = []) {
  if (options == null) {
    return undefined
  }
  const rest = {}
  for (const [name, value] of Object.entries(options)) {
    if (!dropped.includes(name) && value !== undefined) {
      rest[name] = value
    }
  }
  return Object.keys(rest).length === 0 ? undefined : JSON.stringify(rest)
}

function bridge(signal) {
  if (signal == null) {
    return { cancellation: undefined, release() {} }
  }
  const cancellation = new native.Cancellation()
  if (signal.aborted) {
    cancellation.cancel()
    return { cancellation, release() {} }
  }
  const onAbort = () => cancellation.cancel()
  signal.addEventListener('abort', onAbort, { once: true })
  return { cancellation, release: () => signal.removeEventListener('abort', onAbort) }
}

function abortError(signal) {
  if (signal.reason !== undefined) {
    return signal.reason
  }
  const error = new Error('The operation was aborted')
  error.name = 'AbortError'
  return error
}

async function search(roots, query, options = {}) {
  const { cancellation, release } = bridge(options.signal)
  try {
    const encoded = await native.searchRepositories(
      encodeRoots(roots),
      JSON.stringify(normalizeQuery(query)),
      encodeOptions(options, ['signal']),
      cancellation,
    )
    if (options.signal?.aborted) {
      throw abortError(options.signal)
    }
    return JSON.parse(encoded)
  } finally {
    release()
  }
}

function searchSync(roots, query, options = {}) {
  const { cancellation, release } = bridge(options.signal)
  try {
    if (options.signal?.aborted) {
      throw abortError(options.signal)
    }
    return JSON.parse(native.searchRepositoriesSync(
      encodeRoots(roots),
      JSON.stringify(normalizeQuery(query)),
      encodeOptions(options, ['signal']),
      cancellation,
    ))
  } finally {
    release()
  }
}

class Index {
  constructor(handle) {
    if (!(handle instanceof native.NativeIndex)) {
      throw new TypeError('use openIndex, buildIndex, or buildIndexSync to create an Index')
    }
    this._native = handle
  }

  get fileCount() {
    return this._native.fileCount
  }

  get revision() {
    return this._native.revision
  }

  status() {
    return JSON.parse(this._native.statusJson())
  }

  buildReport() {
    const encoded = this._native.buildReportJson()
    return encoded == null ? undefined : JSON.parse(encoded)
  }

  save(path) {
    this._native.save(path)
    return this
  }

  async search(query, options) {
    return JSON.parse(await this._native.searchJson(
      JSON.stringify(normalizeQuery(query)),
      encodeOptions(options, ['signal']),
    ))
  }

  searchSync(query, options) {
    return JSON.parse(this._native.searchJsonSync(
      JSON.stringify(normalizeQuery(query)),
      encodeOptions(options, ['signal']),
    ))
  }

  applyEvents(rootIndex, events, options) {
    if (!Array.isArray(events)) {
      throw new TypeError('events must be an array of { path, kind }')
    }
    return JSON.parse(this._native.updateEventsJson(
      rootIndex,
      JSON.stringify(events),
      encodeOptions(options, ['path', 'signal']),
    ))
  }

  rebuild(options) {
    return JSON.parse(this._native.rebuildJson(encodeOptions(options, ['path', 'signal'])))
  }
}

function openIndex(path, options) {
  return new Index(native.NativeIndex.open(path, encodeOptions(options, ['path', 'signal'])))
}

async function buildIndex(roots, options = {}) {
  const { cancellation, release } = bridge(options.signal)
  try {
    const handle = await native.buildIndex(
      encodeRoots(roots),
      encodeOptions(options, ['path', 'signal']),
      options.path,
      cancellation,
    )
    if (options.signal?.aborted) {
      throw abortError(options.signal)
    }
    return new Index(handle)
  } finally {
    release()
  }
}

function buildIndexSync(roots, options = {}) {
  const { cancellation, release } = bridge(options.signal)
  try {
    if (options.signal?.aborted) {
      throw abortError(options.signal)
    }
    return new Index(native.NativeIndex.buildSync(
      encodeRoots(roots),
      encodeOptions(options, ['path', 'signal']),
      options.path,
      cancellation,
    ))
  } finally {
    release()
  }
}

module.exports = { search, searchSync, buildIndex, buildIndexSync, openIndex, Index }
