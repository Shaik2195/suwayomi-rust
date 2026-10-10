use async_trait::async_trait;
use boa_engine::property::PropertyKey;
use boa_engine::string::JsString;
use boa_engine::{Context, Source};
use std::thread;
use suwayomi_core::error::{Result, SuwayomiError};
use suwayomi_core::models::{Chapter, Filter, Manga, MangaPage, Page};
use suwayomi_core::traits::MangaSource;
use tokio::sync::oneshot;

enum JsCommand {
    LoadScript(String, oneshot::Sender<Result<()>>),
    GetPopularManga(i32, oneshot::Sender<Result<MangaPage>>),
    GetLatestUpdates(i32, oneshot::Sender<Result<MangaPage>>),
    SearchManga(String, Vec<Filter>, i32, oneshot::Sender<Result<MangaPage>>),
    GetMangaDetails(Manga, oneshot::Sender<Result<Manga>>),
    GetChapterList(Manga, oneshot::Sender<Result<Vec<Chapter>>>),
    GetPageList(Chapter, oneshot::Sender<Result<Vec<Page>>>),
    GetFilters(oneshot::Sender<Result<Vec<Filter>>>),
}

#[derive(Clone)]
pub struct JsExtensionRuntime {
    sender: std::sync::mpsc::Sender<JsCommand>,
}

impl JsExtensionRuntime {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<JsCommand>();

        thread::spawn(move || {
            let mut context = Context::default();

            while let Ok(cmd) = rx.recv() {
                match cmd {
                    JsCommand::LoadScript(script, reply) => {
                        let source = Source::from_bytes(&script);
                        let res = context.eval(source).map(|_| ()).map_err(|e| {
                            SuwayomiError::Extension(format!("Failed to evaluate script: {:?}", e))
                        });
                        let _ = reply.send(res);
                    }
                    JsCommand::GetPopularManga(page, reply) => {
                        let res = Self::handle_get_popular_manga(&mut context, page);
                        let _ = reply.send(res);
                    }
                    JsCommand::GetLatestUpdates(page, reply) => {
                        let res = Self::handle_get_latest_updates(&mut context, page);
                        let _ = reply.send(res);
                    }
                    JsCommand::SearchManga(query, filters, page, reply) => {
                        let res = Self::handle_search_manga(&mut context, query, filters, page);
                        let _ = reply.send(res);
                    }
                    JsCommand::GetMangaDetails(manga, reply) => {
                        let res = Self::handle_get_manga_details(&mut context, manga);
                        let _ = reply.send(res);
                    }
                    JsCommand::GetChapterList(manga, reply) => {
                        let res = Self::handle_get_chapter_list(&mut context, manga);
                        let _ = reply.send(res);
                    }
                    JsCommand::GetPageList(chapter, reply) => {
                        let res = Self::handle_get_page_list(&mut context, chapter);
                        let _ = reply.send(res);
                    }
                    JsCommand::GetFilters(reply) => {
                        let res = Self::handle_get_filters(&mut context);
                        let _ = reply.send(res);
                    }
                }
            }
        });

        Self { sender: tx }
    }

    fn call_js_and_deserialize<T: serde::de::DeserializeOwned>(
        context: &mut Context,
        function_name: &str,
        args: &[boa_engine::JsValue],
    ) -> Result<T> {
        let global = context.global_object().clone();

        let func_key: PropertyKey = JsString::from(function_name).into();
        let js_fn = global.get(func_key, context).map_err(|e| {
            SuwayomiError::Extension(format!(
                "Failed to get global function {}: {:?}",
                function_name, e
            ))
        })?;

        if !js_fn.is_callable() {
            return Err(SuwayomiError::Extension(format!(
                "{} is not a function",
                function_name
            )));
        }

        let result = js_fn
            .as_callable()
            .unwrap()
            .call(&boa_engine::JsValue::undefined(), args, context)
            .map_err(|e| {
                SuwayomiError::Extension(format!("Execution failed for {}: {:?}", function_name, e))
            })?;

        let json_key: PropertyKey = JsString::from("JSON").into();
        let json = context
            .global_object()
            .get(json_key, context)
            .map_err(|e| SuwayomiError::Extension(format!("Failed to get JSON object: {:?}", e)))?;

        let stringify_key: PropertyKey = JsString::from("stringify").into();
        let stringify = json
            .as_object()
            .unwrap()
            .get(stringify_key, context)
            .map_err(|e| {
                SuwayomiError::Extension(format!("Failed to get JSON.stringify: {:?}", e))
            })?;

        let json_str_val = stringify
            .as_callable()
            .unwrap()
            .call(&json, &[result], context)
            .map_err(|e| SuwayomiError::Extension(format!("JSON.stringify failed: {:?}", e)))?;

        let json_str = json_str_val
            .as_string()
            .ok_or_else(|| SuwayomiError::Extension("Result is not a string".to_string()))?
            .to_std_string_escaped();

        serde_json::from_str(&json_str).map_err(|e| {
            SuwayomiError::Extension(format!(
                "Failed to deserialize return value of {}: {}",
                function_name, e
            ))
        })
    }

    fn serialize_to_js_value<T: serde::Serialize>(
        context: &mut Context,
        value: &T,
    ) -> Result<boa_engine::JsValue> {
        let json_str = serde_json::to_string(value).map_err(|e| {
            SuwayomiError::Extension(format!("Failed to serialize argument: {}", e))
        })?;

        let json_key: PropertyKey = JsString::from("JSON").into();
        let json = context
            .global_object()
            .get(json_key, context)
            .map_err(|e| SuwayomiError::Extension(format!("Failed to get JSON object: {:?}", e)))?;

        let parse_key: PropertyKey = JsString::from("parse").into();
        let parse = json
            .as_object()
            .unwrap()
            .get(parse_key, context)
            .map_err(|e| SuwayomiError::Extension(format!("Failed to get JSON.parse: {:?}", e)))?;

        let js_str = boa_engine::JsValue::new(JsString::from(json_str));

        parse
            .as_callable()
            .unwrap()
            .call(&json, &[js_str], context)
            .map_err(|e| SuwayomiError::Extension(format!("JSON.parse failed: {:?}", e)))
    }

    fn handle_get_popular_manga(context: &mut Context, page: i32) -> Result<MangaPage> {
        Self::call_js_and_deserialize(
            context,
            "get_popular_manga",
            &[boa_engine::JsValue::new(page)],
        )
    }

    fn handle_get_latest_updates(context: &mut Context, page: i32) -> Result<MangaPage> {
        Self::call_js_and_deserialize(
            context,
            "get_latest_updates",
            &[boa_engine::JsValue::new(page)],
        )
    }

    fn handle_search_manga(
        context: &mut Context,
        query: String,
        filters: Vec<Filter>,
        page: i32,
    ) -> Result<MangaPage> {
        let query_val = boa_engine::JsValue::new(JsString::from(query));
        let filters_val = Self::serialize_to_js_value(context, &filters)?;
        let page_val = boa_engine::JsValue::new(page);

        Self::call_js_and_deserialize(context, "search_manga", &[query_val, filters_val, page_val])
    }

    fn handle_get_manga_details(context: &mut Context, manga: Manga) -> Result<Manga> {
        let manga_val = Self::serialize_to_js_value(context, &manga)?;
        Self::call_js_and_deserialize(context, "get_manga_details", &[manga_val])
    }

    fn handle_get_chapter_list(context: &mut Context, manga: Manga) -> Result<Vec<Chapter>> {
        let manga_val = Self::serialize_to_js_value(context, &manga)?;
        Self::call_js_and_deserialize(context, "get_chapter_list", &[manga_val])
    }

    fn handle_get_page_list(context: &mut Context, chapter: Chapter) -> Result<Vec<Page>> {
        let chapter_val = Self::serialize_to_js_value(context, &chapter)?;
        Self::call_js_and_deserialize(context, "get_page_list", &[chapter_val])
    }

    fn handle_get_filters(context: &mut Context) -> Result<Vec<Filter>> {
        Self::call_js_and_deserialize(context, "get_filters", &[])
    }

    pub async fn load_script(&self, script: &str) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(JsCommand::LoadScript(script.to_string(), tx))
            .map_err(|_| SuwayomiError::Internal("Runtime thread died".to_string()))?;
        rx.await
            .map_err(|_| SuwayomiError::Internal("Failed to receive response".to_string()))?
    }
}

pub struct JsMangaSource {
    runtime: JsExtensionRuntime,
}

impl JsMangaSource {
    pub fn new(runtime: JsExtensionRuntime) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl MangaSource for JsMangaSource {
    async fn get_popular_manga(&self, page: i32) -> Result<MangaPage> {
        let (tx, rx) = oneshot::channel();
        self.runtime
            .sender
            .send(JsCommand::GetPopularManga(page, tx))
            .map_err(|_| SuwayomiError::Internal("Runtime thread died".to_string()))?;
        rx.await
            .map_err(|_| SuwayomiError::Internal("Failed to receive response".to_string()))?
    }

    async fn get_latest_updates(&self, _page: i32) -> Result<MangaPage> {
        let (tx, rx) = oneshot::channel();
        self.runtime
            .sender
            .send(JsCommand::GetLatestUpdates(_page, tx))
            .map_err(|_| SuwayomiError::Internal("Runtime thread died".to_string()))?;
        rx.await
            .map_err(|_| SuwayomiError::Internal("Failed to receive response".to_string()))?
    }

    async fn search_manga(
        &self,
        _query: &str,
        _filters: &[Filter],
        _page: i32,
    ) -> Result<MangaPage> {
        let (tx, rx) = oneshot::channel();
        self.runtime
            .sender
            .send(JsCommand::SearchManga(
                _query.to_string(),
                _filters.to_vec(),
                _page,
                tx,
            ))
            .map_err(|_| SuwayomiError::Internal("Runtime thread died".to_string()))?;
        rx.await
            .map_err(|_| SuwayomiError::Internal("Failed to receive response".to_string()))?
    }

    async fn get_manga_details(&self, _manga: Manga) -> Result<Manga> {
        let (tx, rx) = oneshot::channel();
        self.runtime
            .sender
            .send(JsCommand::GetMangaDetails(_manga, tx))
            .map_err(|_| SuwayomiError::Internal("Runtime thread died".to_string()))?;
        rx.await
            .map_err(|_| SuwayomiError::Internal("Failed to receive response".to_string()))?
    }

    async fn get_chapter_list(&self, _manga: &Manga) -> Result<Vec<Chapter>> {
        let (tx, rx) = oneshot::channel();
        self.runtime
            .sender
            .send(JsCommand::GetChapterList(_manga.clone(), tx))
            .map_err(|_| SuwayomiError::Internal("Runtime thread died".to_string()))?;
        rx.await
            .map_err(|_| SuwayomiError::Internal("Failed to receive response".to_string()))?
    }

    async fn get_page_list(&self, _chapter: &Chapter) -> Result<Vec<Page>> {
        let (tx, rx) = oneshot::channel();
        self.runtime
            .sender
            .send(JsCommand::GetPageList(_chapter.clone(), tx))
            .map_err(|_| SuwayomiError::Internal("Runtime thread died".to_string()))?;
        rx.await
            .map_err(|_| SuwayomiError::Internal("Failed to receive response".to_string()))?
    }

    async fn get_filters(&self) -> Result<Vec<Filter>> {
        let (tx, rx) = oneshot::channel();
        self.runtime
            .sender
            .send(JsCommand::GetFilters(tx))
            .map_err(|_| SuwayomiError::Internal("Runtime thread died".to_string()))?;
        rx.await
            .map_err(|_| SuwayomiError::Internal("Failed to receive response".to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use suwayomi_core::models::MangaStatus;

    #[tokio::test]
    async fn test_js_runtime_eval() {
        let runtime = JsExtensionRuntime::new();

        // Load a mock script
        let script = r#"
            function get_popular_manga(page) {
                return {
                    manga_list: [
                        {
                            id: 1,
                            source_id: 1,
                            url: "/manga/1",
                            title: "JS Manga",
                            status: "Ongoing",
                            update_strategy: 0,
                            initialized: false
                        }
                    ],
                    has_next_page: false
                };
            }
        "#;

        let res = runtime.load_script(script).await;
        assert!(res.is_ok());

        let manga_source = JsMangaSource::new(runtime);
        let popular = manga_source.get_popular_manga(1).await;

        assert!(popular.is_ok());
        let page = popular.unwrap();
        assert_eq!(page.manga_list.len(), 1);
        assert_eq!(page.manga_list[0].title, "JS Manga");
        assert_eq!(page.manga_list[0].status, MangaStatus::Ongoing);
    }

    #[tokio::test]
    async fn test_js_runtime_eval_full() {
        let runtime = JsExtensionRuntime::new();

        // Load a mock script
        let script = r#"
            function get_popular_manga(page) {
                return {
                    manga_list: [
                        {
                            id: 1,
                            source_id: 1,
                            url: "/manga/1",
                            title: "JS Manga",
                            status: "Ongoing",
                            update_strategy: 0,
                            initialized: false
                        }
                    ],
                    has_next_page: false
                };
            }

            function get_latest_updates(page) {
                return {
                    manga_list: [
                        {
                            id: 2,
                            source_id: 1,
                            url: "/manga/2",
                            title: "Latest Manga",
                            status: "Completed",
                            update_strategy: 0,
                            initialized: true
                        }
                    ],
                    has_next_page: true
                };
            }

            function search_manga(query, filters, page) {
                return {
                    manga_list: [
                        {
                            id: 3,
                            source_id: 1,
                            url: "/manga/3",
                            title: "Search result " + query,
                            status: "Unknown",
                            update_strategy: 0,
                            initialized: false
                        }
                    ],
                    has_next_page: false
                };
            }

            function get_manga_details(manga) {
                manga.description = "Detailed description";
                manga.author = "Test Author";
                return manga;
            }

            function get_chapter_list(manga) {
                return [
                    {
                        id: 10,
                        manga_id: manga.id,
                        url: "/chapter/10",
                        name: "Chapter 1",
                        date_upload: 0,
                        chapter_number: 1.0,
                        read: false,
                        bookmark: false,
                        last_page_read: 0,
                        date_fetch: 0,
                        source_order: 1
                    }
                ];
            }

            function get_page_list(chapter) {
                return [
                    {
                        index: 0,
                        url: "/page/0",
                        status: "Ready"
                    }
                ];
            }

            function get_filters() {
                return [
                    {
                        name: "Genre",
                        state: { Text: "Action" }
                    }
                ];
            }
        "#;

        let res = runtime.load_script(script).await;
        assert!(res.is_ok());

        let manga_source = JsMangaSource::new(runtime);

        // test get_popular_manga
        let popular = manga_source.get_popular_manga(1).await.unwrap();
        assert_eq!(popular.manga_list.len(), 1);
        assert_eq!(popular.manga_list[0].title, "JS Manga");
        assert_eq!(popular.manga_list[0].status, MangaStatus::Ongoing);

        // test get_latest_updates
        let latest = manga_source.get_latest_updates(1).await.unwrap();
        assert_eq!(latest.manga_list.len(), 1);
        assert_eq!(latest.manga_list[0].title, "Latest Manga");
        assert_eq!(latest.manga_list[0].status, MangaStatus::Completed);

        // test search_manga
        let search = manga_source
            .search_manga("test query", &[], 1)
            .await
            .unwrap();
        assert_eq!(search.manga_list.len(), 1);
        assert_eq!(search.manga_list[0].title, "Search result test query");

        // test get_manga_details
        let detailed = manga_source
            .get_manga_details(latest.manga_list[0].clone())
            .await
            .unwrap();
        assert_eq!(
            detailed.description.as_deref(),
            Some("Detailed description")
        );
        assert_eq!(detailed.author.as_deref(), Some("Test Author"));

        // test get_chapter_list
        let chapters = manga_source.get_chapter_list(&detailed).await.unwrap();
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].manga_id, detailed.id);

        // test get_page_list
        let pages = manga_source.get_page_list(&chapters[0]).await.unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/page/0");

        // test get_filters
        let filters = manga_source.get_filters().await.unwrap();
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].name, "Genre");
    }
}
