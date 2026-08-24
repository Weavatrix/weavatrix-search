#![deny(unsafe_op_in_unsafe_fn)]

mod index;
mod options;
mod view;

use napi::bindgen_prelude::AsyncTask;
use napi::{Env, Error, Result, Status, Task};
use napi_derive::napi;
use options::{DecodedSearch, decode_query, decode_roots, decode_search};
use std::path::PathBuf;
use view::SearchReportView;
use weavatrix_search::{CancellationToken, SearchQuery, SearchReport, Searcher};

pub use index::{BuildIndexTask, IndexSearchTask, NativeIndex, build_index};

/// A cooperative cancellation handle bridged from a JavaScript `AbortSignal`.
#[napi]
pub struct Cancellation {
    token: CancellationToken,
}

#[napi]
impl Cancellation {
    #[napi(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    #[napi]
    pub fn cancel(&self) {
        self.token.cancel();
    }

    #[napi(getter)]
    pub fn cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    pub(crate) fn token(&self) -> CancellationToken {
        self.token.clone()
    }
}

impl Default for Cancellation {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn token_of(cancellation: Option<&Cancellation>) -> Option<CancellationToken> {
    cancellation.map(Cancellation::token)
}

pub struct SearchTask {
    request: Option<(Vec<PathBuf>, SearchQuery, DecodedSearch)>,
}

impl Task for SearchTask {
    type Output = String;
    type JsValue = String;

    fn compute(&mut self) -> Result<Self::Output> {
        let (roots, query, decoded) = self
            .request
            .take()
            .ok_or_else(|| search_error("search task was already executed"))?;
        run(roots, query, decoded)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

#[napi(js_name = "searchRepositories")]
pub fn search_repositories(
    roots_json: String,
    query_json: String,
    options_json: Option<String>,
    cancellation: Option<&Cancellation>,
) -> Result<AsyncTask<SearchTask>> {
    let roots = decode_roots(&roots_json)?;
    let decoded = decode_search(&roots, options_json.as_deref(), token_of(cancellation))?;
    let query = decode_query(&query_json)?;
    Ok(AsyncTask::new(SearchTask {
        request: Some((roots, query, decoded)),
    }))
}

#[napi(js_name = "searchRepositoriesSync")]
pub fn search_repositories_sync(
    roots_json: String,
    query_json: String,
    options_json: Option<String>,
    cancellation: Option<&Cancellation>,
) -> Result<String> {
    let roots = decode_roots(&roots_json)?;
    let decoded = decode_search(&roots, options_json.as_deref(), token_of(cancellation))?;
    run(roots, decode_query(&query_json)?, decoded)
}

fn run(roots: Vec<PathBuf>, query: SearchQuery, decoded: DecodedSearch) -> Result<String> {
    let mut iterator = roots.into_iter();
    let first = iterator
        .next()
        .ok_or_else(|| search_error("at least one search root is required"))?;
    let report = Searcher::new(first, query)
        .extend_roots(iterator)
        .options(decoded.options)
        .scan_options(decoded.scan)
        .search()
        .map_err(search_error)?;
    encode_report(&report)
}

pub(crate) fn encode_report(report: &SearchReport) -> Result<String> {
    serde_json::to_string(&SearchReportView::new(report)).map_err(json_error)
}

pub(crate) fn json_error(error: serde_json::Error) -> Error {
    Error::new(Status::GenericFailure, error.to_string())
}

pub(crate) fn search_error(error: impl core::fmt::Display) -> Error {
    Error::new(Status::GenericFailure, error.to_string())
}
