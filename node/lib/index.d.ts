export type Query =
  | string
  | { literal: string }
  | { regex: string }
  | { any: Query[] }

export interface ArchiveOptions {
  enabled?: boolean
  maxArchiveBytes?: number
  maxEntryBytes?: number
  maxExpandedBytes?: number
  maxEntries?: number
  maxDecoderMemoryBytes?: number
}

/** Scanner adjustments layered on the profile recommended for the roots. */
export interface ScanOptions {
  extensions?: string[]
  overrideRules?: string[]
  ignoreFiles?: string[]
  skipHidden?: boolean
  parallelism?: number
  maxEntries?: number
  maxTotalBytes?: number
  timeoutMs?: number
}

export interface SearchOptions {
  case?: 'sensitive' | 'insensitive' | 'smart'
  beforeContext?: number
  afterContext?: number
  maxResults?: number
  maxWarnings?: number
  fileEvidence?: 'none' | 'matched' | 'all'
  maxFileEvidence?: number
  maxFileBytes?: number
  maxLineBytes?: number
  maxMultilineBytes?: number
  /** Non-mutating replacement preview; `$0`, `$1`, `${name}`, and `$$`. */
  replacement?: string
  maxReplacementBytes?: number
  mode?: 'line' | 'multiline'
  resultMode?: 'matches' | 'count' | 'files' | 'quiet'
  /** `auto`, `utf8`, `utf16le`, `utf16be`, or an `encoding_rs` label. */
  encoding?: string
  binary?: 'skip' | 'search'
  errorPolicy?: 'continue' | 'abort'
  archives?: ArchiveOptions
  scan?: ScanOptions
  /** Cancels an in-flight search; the promise rejects with `AbortError`. */
  signal?: AbortSignal
}

export interface MatchSpan { patternIndex: number; start: number; end: number }
export interface ContextLine { lineNumber: number; text: string; lossy: boolean }

export interface SearchMatch {
  rootIndex: number
  path: string
  lineNumber: number
  endLineNumber: number
  decodedByteOffset: number
  sourceByteOffset: number | null
  line: string
  replacementPreview: string | null
  spans: MatchSpan[]
  before: ContextLine[]
  after: ContextLine[]
  encoding: string
  lossy: boolean
  archive: boolean
}

export type WarningKind = 'binary' | 'encoding' | 'line-too-long' | 'archive' | 'limit'
export interface SearchWarning { path: string; kind: WarningKind; message: string }

export interface MatchedFile {
  rootIndex: number
  path: string
  matchingLines: number
  occurrences: number
  archive: boolean
}

export interface SourceFileEvidence {
  rootIndex: number
  path: string
  sourceBytes: number
  totalLines: number
  matchingLines: number
  occurrences: number
  encoding: string
  lossy: boolean
  archive: boolean
}

export type ScanTermination = 'max-entries' | 'max-total-bytes' | 'timeout' | 'cancelled'

export interface ScanRootReport {
  root: string
  revision: string
  discovered: number
  completed: number
  bytesEmitted: number
  complete: boolean
  stopped: boolean
  termination: ScanTermination | null
  portable: boolean
}

export interface ScanReport { complete: boolean; cancelled: boolean; roots: ScanRootReport[] }

export interface IndexSearchEvidence {
  revision: string
  indexedFiles: number
  candidateFiles: number
  prefiltered: boolean
}

export interface SearchReport {
  backend: 'filesystem' | 'persistent-index' | 'live-index'
  index: IndexSearchEvidence | null
  roots: string[]
  resultMode: 'matches' | 'count' | 'files' | 'quiet'
  matches: SearchMatch[]
  matchingLines: number
  occurrences: number
  filesWithMatches: number
  filesSearched: number
  bytesSearched: number
  truncated: boolean
  warnings: SearchWarning[]
  matchedFiles: MatchedFile[]
  fileEvidence: SourceFileEvidence[]
  fileEvidenceTruncated: boolean
  warningsDropped: number
  scan: ScanReport
}

export interface IndexOptions {
  maxEntries?: number
  maxContentBytes?: number
  maxIndexBytes?: number
  maxPathBytes?: number
  buildParallelism?: number
  searchParallelism?: number
  scan?: ScanOptions
}

export interface BuildIndexOptions extends IndexOptions {
  /** Saves the snapshot atomically to this path as part of the build. */
  path?: string
  signal?: AbortSignal
}

export interface IndexStatus { roots: string[]; files: number; contentBytes: number; revision: string }

export interface IndexBuildReport {
  roots: string[]
  files: number
  contentBytes: number
  revision: string
  scan: ScanReport
}

export interface IndexUpdateReport {
  added: number
  updated: number
  removed: number
  files: number
  contentBytes: number
  revision: string
  fullRebuild: boolean
  changedScan: ScanRootReport | null
}

export type WatchEventKind =
  | 'create'
  | 'modify'
  | 'remove'
  | 'renameFrom'
  | 'renameTo'
  | 'directory'
  | 'rescan'

export interface WatchEvent { path: string; kind: WatchEventKind }

export declare class Index {
  readonly fileCount: number
  readonly revision: string
  status(): IndexStatus
  buildReport(): IndexBuildReport | undefined
  save(path: string): this
  search(query: Query, options?: SearchOptions): Promise<SearchReport>
  searchSync(query: Query, options?: SearchOptions): SearchReport
  applyEvents(rootIndex: number, events: WatchEvent[], options?: IndexOptions): IndexUpdateReport
  rebuild(options?: IndexOptions): IndexUpdateReport
}

export declare function search(
  roots: string | string[],
  query: Query,
  options?: SearchOptions,
): Promise<SearchReport>
export declare function searchSync(
  roots: string | string[],
  query: Query,
  options?: SearchOptions,
): SearchReport
export declare function buildIndex(
  roots: string | string[],
  options?: BuildIndexOptions,
): Promise<Index>
export declare function buildIndexSync(
  roots: string | string[],
  options?: BuildIndexOptions,
): Index
export declare function openIndex(path: string, options?: IndexOptions): Index
