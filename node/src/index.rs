//! The persistent-index class exposed to Node.js and Bun.

use crate::options::{
    decode_events, decode_index, decode_query, decode_roots, decode_search_options,
};
use crate::view::{BuildReportView, StatusView, UpdateReportView};
use crate::{Cancellation, encode_report, json_error, search_error, token_of};
use napi::bindgen_prelude::AsyncTask;
use napi::{Env, Result, Task};
use napi_derive::napi;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use weavatrix_search::{
    IndexOptions, PersistentIndex, ScanOptions, SearchOptions, SearchQuery, WatchPlan,
};

type Shared = Arc<RwLock<PersistentIndex>>;

/// An opened or freshly built persistent content index.
#[napi]
pub struct NativeIndex {
    inner: Shared,
    build_report: Option<String>,
}

#[napi]
impl NativeIndex {
    /// Opens a saved index without rescanning any repository.
    #[napi(factory)]
    pub fn open(path: String, options_json: Option<String>) -> Result<Self> {
        let (options, _) = decode_index(options_json.as_deref(), None)?;
        let index = PersistentIndex::open(path, options).map_err(search_error)?;
        Ok(Self::wrap(index, None))
    }

    /// Builds a complete index on the calling thread.
    #[napi(factory)]
    pub fn build_sync(
        roots_json: String,
        options_json: Option<String>,
        save_path: Option<String>,
        cancellation: Option<&Cancellation>,
    ) -> Result<Self> {
        let roots = decode_roots(&roots_json)?;
        let (options, scan) = decode_index(options_json.as_deref(), token_of(cancellation))?;
        build(roots, options, scan, save_path.map(PathBuf::from))
    }

    #[napi(getter)]
    pub fn file_count(&self) -> Result<u32> {
        let index = self.read()?;
        u32::try_from(index.len()).map_err(search_error)
    }

    #[napi(getter)]
    pub fn revision(&self) -> Result<String> {
        Ok(self.read()?.revision().to_owned())
    }

    /// Returns the cheap health status of the resident snapshot.
    #[napi]
    pub fn status_json(&self) -> Result<String> {
        let index = self.read()?;
        serde_json::to_string(&StatusView::new(&index.status())).map_err(json_error)
    }

    /// Returns the build report when this handle came from a build.
    #[napi]
    pub fn build_report_json(&self) -> Option<String> {
        self.build_report.clone()
    }

    /// Saves the resident snapshot atomically.
    #[napi]
    pub fn save(&self, path: String) -> Result<()> {
        self.read()?.save(path).map_err(search_error)
    }

    #[napi(js_name = "searchJson")]
    pub fn search_json(
        &self,
        query_json: String,
        options_json: Option<String>,
    ) -> Result<AsyncTask<IndexSearchTask>> {
        Ok(AsyncTask::new(IndexSearchTask {
            request: Some((
                Arc::clone(&self.inner),
                decode_query(&query_json)?,
                decode_search_options(options_json.as_deref())?,
            )),
        }))
    }

    #[napi(js_name = "searchJsonSync")]
    pub fn search_json_sync(
        &self,
        query_json: String,
        options_json: Option<String>,
    ) -> Result<String> {
        query(
            &self.inner,
            decode_query(&query_json)?,
            decode_search_options(options_json.as_deref())?,
        )
    }

    /// Applies watcher events for one root and returns the update report.
    #[napi]
    pub fn update_events_json(
        &self,
        root_index: u32,
        events_json: String,
        options_json: Option<String>,
    ) -> Result<String> {
        let events = decode_events(&events_json)?;
        let (_, scan) = decode_index(options_json.as_deref(), None)?;
        let mut index = self.write()?;
        let report = index
            .update_events(root_index as usize, events, scan)
            .map_err(search_error)?;
        serde_json::to_string(&UpdateReportView::new(&report)).map_err(json_error)
    }

    /// Rescans every indexed root and replaces the snapshot in place.
    #[napi]
    pub fn rebuild_json(&self, options_json: Option<String>) -> Result<String> {
        let (_, scan) = decode_index(options_json.as_deref(), None)?;
        let plan = WatchPlan {
            full_rescan: true,
            ..WatchPlan::default()
        };
        let mut index = self.write()?;
        let report = index.update(0, &plan, scan).map_err(search_error)?;
        serde_json::to_string(&UpdateReportView::new(&report)).map_err(json_error)
    }

    fn wrap(index: PersistentIndex, build_report: Option<String>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(index)),
            build_report,
        }
    }

    fn read(&self) -> Result<std::sync::RwLockReadGuard<'_, PersistentIndex>> {
        self.inner
            .read()
            .map_err(|_| search_error("index lock was poisoned"))
    }

    fn write(&self) -> Result<std::sync::RwLockWriteGuard<'_, PersistentIndex>> {
        self.inner
            .write()
            .map_err(|_| search_error("index lock was poisoned"))
    }
}

/// Builds a complete index off the JavaScript event loop.
#[napi(js_name = "buildIndex")]
pub fn build_index(
    roots_json: String,
    options_json: Option<String>,
    save_path: Option<String>,
    cancellation: Option<&Cancellation>,
) -> Result<AsyncTask<BuildIndexTask>> {
    let roots = decode_roots(&roots_json)?;
    let (options, scan) = decode_index(options_json.as_deref(), token_of(cancellation))?;
    Ok(AsyncTask::new(BuildIndexTask {
        request: Some((roots, options, scan, save_path.map(PathBuf::from))),
    }))
}

pub struct BuildIndexTask {
    request: Option<(Vec<PathBuf>, IndexOptions, ScanOptions, Option<PathBuf>)>,
}

impl Task for BuildIndexTask {
    type Output = NativeIndex;
    type JsValue = NativeIndex;

    fn compute(&mut self) -> Result<Self::Output> {
        let (roots, options, scan, save_path) = self
            .request
            .take()
            .ok_or_else(|| search_error("index build task was already executed"))?;
        build(roots, options, scan, save_path)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

pub struct IndexSearchTask {
    request: Option<(Shared, SearchQuery, SearchOptions)>,
}

impl Task for IndexSearchTask {
    type Output = String;
    type JsValue = String;

    fn compute(&mut self) -> Result<Self::Output> {
        let (index, search, options) = self
            .request
            .take()
            .ok_or_else(|| search_error("index search task was already executed"))?;
        query(&index, search, options)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

fn build(
    roots: Vec<PathBuf>,
    options: IndexOptions,
    scan: ScanOptions,
    save_path: Option<PathBuf>,
) -> Result<NativeIndex> {
    let (index, report) = match save_path {
        Some(path) => PersistentIndex::build_and_save(path, roots, scan, options),
        None => PersistentIndex::build(roots, scan, options),
    }
    .map_err(search_error)?;
    let encoded = serde_json::to_string(&BuildReportView::new(&report)).map_err(json_error)?;
    Ok(NativeIndex::wrap(index, Some(encoded)))
}

fn query(index: &Shared, search: SearchQuery, options: SearchOptions) -> Result<String> {
    let index = index
        .read()
        .map_err(|_| search_error("index lock was poisoned"))?;
    let report = index.search(search, options).map_err(search_error)?;
    encode_report(&report)
}
