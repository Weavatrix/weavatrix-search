//! Decoding of the JSON policy documents handed over the Node boundary.

use napi::{Error, Result, Status};
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;
use weavatrix_scan::WatchEventKind;
use weavatrix_search::{
    ArchiveOptions, BinaryPolicy, CancellationToken, CaseMode, EncodingMode, FileEvidenceMode,
    IndexOptions, ResultMode, ScanOptions, SearchErrorPolicy, SearchMode, SearchOptions,
    SearchQuery, WatchEvent, recommended_scan_options,
};

/// A query as written in JavaScript: `{ literal }`, `{ regex }`, or `{ any }`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum NodeQuery {
    Literal(String),
    Regex(String),
    Any(Vec<NodeQuery>),
}

impl From<NodeQuery> for SearchQuery {
    fn from(value: NodeQuery) -> Self {
        match value {
            NodeQuery::Literal(pattern) => Self::Literal(pattern),
            NodeQuery::Regex(pattern) => Self::Regex(pattern),
            NodeQuery::Any(queries) => Self::Any(queries.into_iter().map(Self::from).collect()),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NodeSearchOptions {
    case: Option<String>,
    before_context: Option<usize>,
    after_context: Option<usize>,
    max_results: Option<usize>,
    max_warnings: Option<usize>,
    file_evidence: Option<String>,
    max_file_evidence: Option<usize>,
    max_file_bytes: Option<u64>,
    max_line_bytes: Option<usize>,
    max_multiline_bytes: Option<u64>,
    replacement: Option<String>,
    max_replacement_bytes: Option<usize>,
    mode: Option<String>,
    result_mode: Option<String>,
    encoding: Option<String>,
    binary: Option<String>,
    error_policy: Option<String>,
    archives: Option<NodeArchiveOptions>,
    scan: Option<NodeScanOptions>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
struct NodeArchiveOptions {
    enabled: Option<bool>,
    max_archive_bytes: Option<u64>,
    max_entry_bytes: Option<u64>,
    max_expanded_bytes: Option<u64>,
    max_entries: Option<usize>,
    max_decoder_memory_bytes: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NodeScanOptions {
    extensions: Vec<String>,
    override_rules: Vec<String>,
    ignore_files: Vec<String>,
    skip_hidden: Option<bool>,
    parallelism: Option<usize>,
    max_entries: Option<u64>,
    max_total_bytes: Option<u64>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NodeIndexOptions {
    max_entries: Option<u64>,
    max_content_bytes: Option<u64>,
    max_index_bytes: Option<u64>,
    max_path_bytes: Option<usize>,
    build_parallelism: Option<usize>,
    search_parallelism: Option<usize>,
    #[serde(default)]
    scan: Option<NodeScanOptions>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NodeWatchEvent {
    path: String,
    kind: String,
}

/// Search policy plus the scanner profile it implies for `roots`.
pub(crate) struct DecodedSearch {
    pub(crate) options: SearchOptions,
    pub(crate) scan: ScanOptions,
}

pub(crate) fn decode_roots(roots_json: &str) -> Result<Vec<PathBuf>> {
    let roots: Vec<String> = serde_json::from_str(roots_json).map_err(invalid)?;
    if roots.is_empty() {
        return Err(Error::new(
            Status::InvalidArg,
            "at least one search root is required",
        ));
    }
    Ok(roots.into_iter().map(PathBuf::from).collect())
}

pub(crate) fn decode_query(query_json: &str) -> Result<SearchQuery> {
    serde_json::from_str::<NodeQuery>(query_json)
        .map(SearchQuery::from)
        .map_err(invalid)
}

pub(crate) fn decode_search(
    roots: &[PathBuf],
    options_json: Option<&str>,
    cancellation: Option<CancellationToken>,
) -> Result<DecodedSearch> {
    let input = parse_options::<NodeSearchOptions>(options_json)?;
    let options = search_options(&input)?;
    let mut scan = recommended_scan_options(roots, &options);
    scan = apply_scan(scan, input.scan.as_ref());
    if let Some(token) = cancellation {
        scan = scan.with_cancellation(token);
    }
    Ok(DecodedSearch { options, scan })
}

pub(crate) fn decode_search_options(options_json: Option<&str>) -> Result<SearchOptions> {
    search_options(&parse_options::<NodeSearchOptions>(options_json)?)
}

pub(crate) fn decode_index(
    options_json: Option<&str>,
    cancellation: Option<CancellationToken>,
) -> Result<(IndexOptions, ScanOptions)> {
    let input = parse_options::<NodeIndexOptions>(options_json)?;
    let mut options = IndexOptions::default();
    if let Some(value) = input.max_entries {
        options = options.with_max_entries(value);
    }
    if let Some(value) = input.max_content_bytes {
        options = options.with_max_content_bytes(value);
    }
    if let Some(value) = input.max_index_bytes {
        options = options.with_max_index_bytes(value);
    }
    if let Some(value) = input.max_path_bytes {
        options = options.with_max_path_bytes(value);
    }
    if let Some(value) = input.build_parallelism {
        options = options.with_build_parallelism(value);
    }
    if let Some(value) = input.search_parallelism {
        options = options.with_search_parallelism(value);
    }
    let mut scan = index_scan_options();
    scan = apply_scan(scan, input.scan.as_ref());
    if let Some(token) = cancellation {
        scan = scan.with_cancellation(token);
    }
    Ok((options, scan))
}

pub(crate) fn decode_events(events_json: &str) -> Result<Vec<WatchEvent>> {
    serde_json::from_str::<Vec<NodeWatchEvent>>(events_json)
        .map_err(invalid)?
        .into_iter()
        .map(|event| Ok(WatchEvent::new(event.path, watch_kind(&event.kind)?)))
        .collect()
}

fn index_scan_options() -> ScanOptions {
    ScanOptions::default()
        .metadata_only()
        .selected_files_only()
        .with_skip_hidden(true)
        .with_content_discovery(weavatrix_scan::ContentDiscoveryMode::BufferedParallel)
        .with_content_validation(weavatrix_scan::ContentValidationPolicy::Fast)
}

fn parse_options<T: Default + for<'de> Deserialize<'de>>(options_json: Option<&str>) -> Result<T> {
    match options_json {
        None => Ok(T::default()),
        Some(raw) => serde_json::from_str(raw).map_err(invalid),
    }
}

fn search_options(input: &NodeSearchOptions) -> Result<SearchOptions> {
    let mut options = SearchOptions::default();
    if let Some(value) = &input.case {
        options = options.with_case(case_mode(value)?);
    }
    if input.before_context.is_some() || input.after_context.is_some() {
        options = options.with_context(
            input.before_context.unwrap_or(0),
            input.after_context.unwrap_or(0),
        );
    }
    if let Some(value) = input.max_results {
        options = options.with_max_results(value);
    }
    if let Some(value) = input.max_warnings {
        options = options.with_max_warnings(value);
    }
    if let Some(value) = &input.file_evidence {
        options = options.with_file_evidence(file_evidence(value)?);
    }
    if let Some(value) = input.max_file_evidence {
        options = options.with_max_file_evidence(value);
    }
    if let Some(value) = input.max_file_bytes {
        options = options.with_max_file_bytes(value);
    }
    if let Some(value) = input.max_line_bytes {
        options = options.with_max_line_bytes(value);
    }
    if let Some(value) = input.max_multiline_bytes {
        options = options.with_max_multiline_bytes(value);
    }
    if let Some(value) = &input.replacement {
        options = options.with_replacement(value.clone());
    }
    if let Some(value) = input.max_replacement_bytes {
        options = options.with_max_replacement_bytes(value);
    }
    if let Some(value) = &input.mode {
        options = options.with_mode(search_mode(value)?);
    }
    if let Some(value) = &input.result_mode {
        options = options.with_result_mode(result_mode(value)?);
    }
    if let Some(value) = &input.encoding {
        options = options.with_encoding(encoding_mode(value));
    }
    if let Some(value) = &input.binary {
        options = options.with_binary_policy(binary_policy(value)?);
    }
    if let Some(value) = &input.error_policy {
        options = options.with_error_policy(error_policy(value)?);
    }
    if let Some(value) = &input.archives {
        options = options.with_archives(archive_options(value));
    }
    Ok(options)
}

fn apply_scan(mut scan: ScanOptions, input: Option<&NodeScanOptions>) -> ScanOptions {
    let Some(input) = input else {
        return scan;
    };
    if !input.extensions.is_empty() {
        scan = scan.with_extensions(&input.extensions);
    }
    if !input.override_rules.is_empty() {
        scan = scan.with_override_rules(&input.override_rules);
    }
    if !input.ignore_files.is_empty() {
        scan = scan.with_ignore_files(&input.ignore_files);
    }
    if let Some(value) = input.skip_hidden {
        scan = scan.with_skip_hidden(value);
    }
    if let Some(value) = input.parallelism {
        scan = scan.with_content_parallelism(value);
    }
    if input.max_entries.is_some() {
        scan = scan.with_max_entries(input.max_entries);
    }
    if input.max_total_bytes.is_some() {
        scan = scan.with_max_total_bytes(input.max_total_bytes);
    }
    if let Some(value) = input.timeout_ms {
        scan = scan.with_timeout(Some(Duration::from_millis(value)));
    }
    scan
}

fn archive_options(input: &NodeArchiveOptions) -> ArchiveOptions {
    let mut archives = ArchiveOptions::default();
    if let Some(value) = input.enabled {
        archives.enabled = value;
    }
    if let Some(value) = input.max_archive_bytes {
        archives.max_archive_bytes = value;
    }
    if let Some(value) = input.max_entry_bytes {
        archives.max_entry_bytes = value;
    }
    if let Some(value) = input.max_expanded_bytes {
        archives.max_expanded_bytes = value;
    }
    if let Some(value) = input.max_entries {
        archives.max_entries = value;
    }
    if let Some(value) = input.max_decoder_memory_bytes {
        archives.max_decoder_memory_bytes = value;
    }
    archives
}

fn case_mode(value: &str) -> Result<CaseMode> {
    match value {
        "sensitive" => Ok(CaseMode::Sensitive),
        "insensitive" => Ok(CaseMode::Insensitive),
        "smart" => Ok(CaseMode::Smart),
        _ => Err(unsupported("case", value)),
    }
}

fn file_evidence(value: &str) -> Result<FileEvidenceMode> {
    match value {
        "none" => Ok(FileEvidenceMode::None),
        "matched" => Ok(FileEvidenceMode::Matched),
        "all" => Ok(FileEvidenceMode::All),
        _ => Err(unsupported("fileEvidence", value)),
    }
}

fn search_mode(value: &str) -> Result<SearchMode> {
    match value {
        "line" => Ok(SearchMode::Line),
        "multiline" => Ok(SearchMode::Multiline),
        _ => Err(unsupported("mode", value)),
    }
}

fn result_mode(value: &str) -> Result<ResultMode> {
    match value {
        "matches" => Ok(ResultMode::Matches),
        "count" => Ok(ResultMode::Count),
        "files" => Ok(ResultMode::Files),
        "quiet" => Ok(ResultMode::Quiet),
        _ => Err(unsupported("resultMode", value)),
    }
}

fn encoding_mode(value: &str) -> EncodingMode {
    match value {
        "auto" => EncodingMode::Auto,
        "utf8" | "utf-8" => EncodingMode::Utf8,
        "utf16le" | "utf-16le" => EncodingMode::Utf16Le,
        "utf16be" | "utf-16be" => EncodingMode::Utf16Be,
        label => EncodingMode::Label(label.to_owned()),
    }
}

fn binary_policy(value: &str) -> Result<BinaryPolicy> {
    match value {
        "skip" => Ok(BinaryPolicy::Skip),
        "search" => Ok(BinaryPolicy::Search),
        _ => Err(unsupported("binary", value)),
    }
}

fn error_policy(value: &str) -> Result<SearchErrorPolicy> {
    match value {
        "continue" => Ok(SearchErrorPolicy::Continue),
        "abort" => Ok(SearchErrorPolicy::Abort),
        _ => Err(unsupported("errorPolicy", value)),
    }
}

fn watch_kind(value: &str) -> Result<WatchEventKind> {
    match value {
        "create" => Ok(WatchEventKind::Create),
        "modify" => Ok(WatchEventKind::Modify),
        "remove" => Ok(WatchEventKind::Remove),
        "renameFrom" => Ok(WatchEventKind::RenameFrom),
        "renameTo" => Ok(WatchEventKind::RenameTo),
        "directory" => Ok(WatchEventKind::Directory),
        "rescan" => Ok(WatchEventKind::Rescan),
        _ => Err(unsupported("watch event kind", value)),
    }
}

fn unsupported(field: &str, value: &str) -> Error {
    Error::new(
        Status::InvalidArg,
        format!("unsupported {field} value: {value}"),
    )
}

fn invalid(error: serde_json::Error) -> Error {
    Error::new(Status::InvalidArg, error.to_string())
}
