//! Borrowed serialization views for search and index reports.

use serde::Serialize;
use std::path::Path;
use weavatrix_scan::{ContentVisitReport, MultiContentVisitReport, ScanTermination};
use weavatrix_search::{
    ContextLine, IndexBuildReport, IndexSearchEvidence, IndexStatus, IndexUpdateReport, MatchSpan,
    MatchedFile, ResultMode, SearchBackend, SearchMatch, SearchReport, SearchWarning,
    SearchWarningKind, SourceFileEvidence,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchReportView<'a> {
    backend: &'static str,
    index: Option<IndexEvidenceView<'a>>,
    roots: Vec<String>,
    result_mode: &'static str,
    matches: Vec<MatchView<'a>>,
    matching_lines: u64,
    occurrences: u64,
    files_with_matches: u64,
    files_searched: u64,
    bytes_searched: u64,
    truncated: bool,
    warnings: Vec<WarningView<'a>>,
    matched_files: Vec<MatchedFileView<'a>>,
    file_evidence: Vec<FileEvidenceView<'a>>,
    file_evidence_truncated: bool,
    warnings_dropped: u64,
    scan: ScanView<'a>,
}

impl<'a> SearchReportView<'a> {
    pub(crate) fn new(report: &'a SearchReport) -> Self {
        Self {
            backend: backend(report.backend),
            index: report.index.as_ref().map(IndexEvidenceView::new),
            roots: report.roots.iter().map(display).collect(),
            result_mode: result_mode(report.result_mode),
            matches: report.matches.iter().map(MatchView::new).collect(),
            matching_lines: report.matching_lines,
            occurrences: report.occurrences,
            files_with_matches: report.files_with_matches,
            files_searched: report.files_searched,
            bytes_searched: report.bytes_searched,
            truncated: report.truncated,
            warnings: report.warnings.iter().map(WarningView::new).collect(),
            matched_files: report
                .matched_files
                .iter()
                .map(MatchedFileView::new)
                .collect(),
            file_evidence: report
                .file_evidence
                .iter()
                .map(FileEvidenceView::new)
                .collect(),
            file_evidence_truncated: report.file_evidence_truncated,
            warnings_dropped: report.warnings_dropped,
            scan: ScanView::new(&report.scan),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexEvidenceView<'a> {
    revision: &'a str,
    indexed_files: u64,
    candidate_files: u64,
    prefiltered: bool,
}

impl<'a> IndexEvidenceView<'a> {
    fn new(evidence: &'a IndexSearchEvidence) -> Self {
        Self {
            revision: &evidence.revision,
            indexed_files: evidence.indexed_files,
            candidate_files: evidence.candidate_files,
            prefiltered: evidence.prefiltered,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MatchView<'a> {
    root_index: usize,
    path: &'a str,
    line_number: u64,
    end_line_number: u64,
    decoded_byte_offset: u64,
    source_byte_offset: Option<u64>,
    line: &'a str,
    replacement_preview: Option<&'a str>,
    spans: Vec<SpanView>,
    before: Vec<ContextView<'a>>,
    after: Vec<ContextView<'a>>,
    encoding: &'a str,
    lossy: bool,
    archive: bool,
}

impl<'a> MatchView<'a> {
    fn new(found: &'a SearchMatch) -> Self {
        Self {
            root_index: found.root_index,
            path: &found.path,
            line_number: found.line_number,
            end_line_number: found.end_line_number,
            decoded_byte_offset: found.decoded_byte_offset,
            source_byte_offset: found.source_byte_offset,
            line: &found.line,
            replacement_preview: found.replacement_preview.as_deref(),
            spans: found.spans.iter().copied().map(SpanView::from).collect(),
            before: found.before.iter().map(ContextView::new).collect(),
            after: found.after.iter().map(ContextView::new).collect(),
            encoding: &found.encoding,
            lossy: found.lossy,
            archive: found.archive,
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct SpanView {
    pattern_index: usize,
    start: usize,
    end: usize,
}

impl From<MatchSpan> for SpanView {
    fn from(span: MatchSpan) -> Self {
        Self {
            pattern_index: span.pattern_index,
            start: span.start,
            end: span.end,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ContextView<'a> {
    line_number: u64,
    text: &'a str,
    lossy: bool,
}

impl<'a> ContextView<'a> {
    fn new(line: &'a ContextLine) -> Self {
        Self {
            line_number: line.line_number,
            text: &line.text,
            lossy: line.lossy,
        }
    }
}

#[derive(Serialize)]
struct WarningView<'a> {
    path: &'a str,
    kind: &'static str,
    message: &'a str,
}

impl<'a> WarningView<'a> {
    fn new(warning: &'a SearchWarning) -> Self {
        Self {
            path: &warning.path,
            kind: warning_kind(warning.kind),
            message: &warning.message,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MatchedFileView<'a> {
    root_index: usize,
    path: &'a str,
    matching_lines: u64,
    occurrences: u64,
    archive: bool,
}

impl<'a> MatchedFileView<'a> {
    fn new(file: &'a MatchedFile) -> Self {
        Self {
            root_index: file.root_index,
            path: &file.path,
            matching_lines: file.matching_lines,
            occurrences: file.occurrences,
            archive: file.archive,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileEvidenceView<'a> {
    root_index: usize,
    path: &'a str,
    source_bytes: u64,
    total_lines: u64,
    matching_lines: u64,
    occurrences: u64,
    encoding: &'a str,
    lossy: bool,
    archive: bool,
}

impl<'a> FileEvidenceView<'a> {
    fn new(evidence: &'a SourceFileEvidence) -> Self {
        Self {
            root_index: evidence.root_index,
            path: &evidence.path,
            source_bytes: evidence.source_bytes,
            total_lines: evidence.total_lines,
            matching_lines: evidence.matching_lines,
            occurrences: evidence.occurrences,
            encoding: &evidence.encoding,
            lossy: evidence.lossy,
            archive: evidence.archive,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanView<'a> {
    complete: bool,
    cancelled: bool,
    roots: Vec<ScanRootView<'a>>,
}

impl<'a> ScanView<'a> {
    pub(crate) fn new(scan: &'a MultiContentVisitReport) -> Self {
        Self {
            complete: scan.reports.iter().all(|report| report.complete),
            cancelled: scan
                .reports
                .iter()
                .any(|report| report.termination == Some(ScanTermination::Cancelled)),
            roots: scan.reports.iter().map(ScanRootView::new).collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanRootView<'a> {
    root: String,
    revision: &'a str,
    discovered: u64,
    completed: u64,
    bytes_emitted: u64,
    complete: bool,
    stopped: bool,
    termination: Option<&'static str>,
    portable: bool,
}

impl<'a> ScanRootView<'a> {
    pub(crate) fn new(report: &'a ContentVisitReport) -> Self {
        Self {
            root: display(&report.root),
            revision: &report.revision,
            discovered: report.discovered,
            completed: report.completed,
            bytes_emitted: report.bytes_emitted,
            complete: report.complete,
            stopped: report.stopped,
            termination: report.termination.map(termination),
            portable: report.portable,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuildReportView<'a> {
    roots: Vec<String>,
    files: u64,
    content_bytes: u64,
    revision: &'a str,
    scan: ScanView<'a>,
}

impl<'a> BuildReportView<'a> {
    pub(crate) fn new(report: &'a IndexBuildReport) -> Self {
        Self {
            roots: report.roots.iter().map(display).collect(),
            files: report.files,
            content_bytes: report.content_bytes,
            revision: &report.revision,
            scan: ScanView::new(&report.scan),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateReportView<'a> {
    added: u64,
    updated: u64,
    removed: u64,
    files: u64,
    content_bytes: u64,
    revision: &'a str,
    full_rebuild: bool,
    changed_scan: Option<ScanRootView<'a>>,
}

impl<'a> UpdateReportView<'a> {
    pub(crate) fn new(report: &'a IndexUpdateReport) -> Self {
        Self {
            added: report.added,
            updated: report.updated,
            removed: report.removed,
            files: report.files,
            content_bytes: report.content_bytes,
            revision: &report.revision,
            full_rebuild: report.full_rebuild,
            changed_scan: report.changed_scan.as_ref().map(ScanRootView::new),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusView<'a> {
    roots: Vec<String>,
    files: u64,
    content_bytes: u64,
    revision: &'a str,
}

impl<'a> StatusView<'a> {
    pub(crate) fn new(status: &'a IndexStatus) -> Self {
        Self {
            roots: status.roots.iter().map(display).collect(),
            files: status.files,
            content_bytes: status.content_bytes,
            revision: &status.revision,
        }
    }
}

fn display(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().into_owned()
}

const fn backend(value: SearchBackend) -> &'static str {
    match value {
        SearchBackend::Filesystem => "filesystem",
        SearchBackend::PersistentIndex => "persistent-index",
        SearchBackend::LiveIndex => "live-index",
    }
}

const fn result_mode(value: ResultMode) -> &'static str {
    match value {
        ResultMode::Matches => "matches",
        ResultMode::Count => "count",
        ResultMode::Files => "files",
        ResultMode::Quiet => "quiet",
    }
}

const fn warning_kind(value: SearchWarningKind) -> &'static str {
    match value {
        SearchWarningKind::Binary => "binary",
        SearchWarningKind::Encoding => "encoding",
        SearchWarningKind::LineTooLong => "line-too-long",
        SearchWarningKind::Archive => "archive",
        SearchWarningKind::Limit => "limit",
    }
}

const fn termination(value: ScanTermination) -> &'static str {
    match value {
        ScanTermination::MaxEntries => "max-entries",
        ScanTermination::MaxTotalBytes => "max-total-bytes",
        ScanTermination::Timeout => "timeout",
        ScanTermination::Cancelled => "cancelled",
    }
}
