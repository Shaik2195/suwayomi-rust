use boa_engine::{Context, Source};
use boa_engine::string::JsString;
use boa_engine::property::PropertyKey;
use suwayomi_core::error::{Result, SuwayomiError};
use suwayomi_core::models::{Chapter, Filter, Manga, MangaPage, Page};
use suwayomi_core::traits::MangaSource;
use async_trait::async_trait;
use std::thread;
use tokio::sync::oneshot;

enum JsCommand {
    LoadScript(String, oneshot::Sender<Result<()>>),
    GetPopularManga(i32, oneshot::Sender<Result<MangaPage>>),
    GetLatestUpdates((), oneshot::Sender<Result<MangaPage>>),
    SearchManga((), (), (), oneshot::Sender<Result<MangaPage>>),
    GetMangaDetails((), oneshot::Sender<Result<Manga>>),
    GetChapterList((), oneshot::Sender<Result<Vec<Chapter>>>),
    GetPageList((), oneshot::Sender<Result<Vec<Page>>>),
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
                        let res = context.eval(source)
                            .map(|_| ())
                            .map_err(|e| SuwayomiError::Extension(format!("Failed to evaluate script: {:?}", e)));
                        let _ = reply.send(res);
                    }
                    JsCommand::GetPopularManga(page, reply) => {
                        let res = Self::handle_get_popular_manga(&mut context, page);
                        let _ = reply.send(res);
                    }
                    JsCommand::GetLatestUpdates(_, reply) => {
                        let _ = reply.send(Err(SuwayomiError::Extension("Not implemented".to_string())));
                    }
                    JsCommand::SearchManga(_, _, _, reply) => {
                        let _ = reply.send(Err(SuwayomiError::Extension("Not implemented".to_string())));
                    }
                    JsCommand::GetMangaDetails(_, reply) => {
                        let _ = reply.send(Err(SuwayomiError::Extension("Not implemented".to_string())));
                    }
                    JsCommand::GetChapterList(_, reply) => {
                        let _ = reply.send(Err(SuwayomiError::Extension("Not implemented".to_string())));
                    }
                    JsCommand::GetPageList(_, reply) => {
                        let _ = reply.send(Err(SuwayomiError::Extension("Not implemented".to_string())));
                    }
                    JsCommand::GetFilters(reply) => {
                        let _ = reply.send(Err(SuwayomiError::Extension("Not implemented".to_string())));
                    }
                }
            }
        });
        
        Self { sender: tx }
    }
    
    fn handle_get_popular_manga(context: &mut Context, page: i32) -> Result<MangaPage> {
        let global = context.global_object().clone();
        
        let get_popular_manga_key: PropertyKey = JsString::from("get_popular_manga").into();
        let get_popular_manga_fn = global.get(get_popular_manga_key, context)
            .map_err(|e| SuwayomiError::Extension(format!("Failed to get global function: {:?}", e)))?;
            
        if !get_popular_manga_fn.is_callable() {
            return Err(SuwayomiError::Extension("get_popular_manga is not a function".to_string()));
        }
        
        let args = [boa_engine::JsValue::new(page)];
        
        let result = get_popular_manga_fn.as_callable().unwrap()
            .call(&boa_engine::JsValue::undefined(), &args, context)
            .map_err(|e| SuwayomiError::Extension(format!("Execution failed: {:?}", e)))?;
            
        let json_key: PropertyKey = JsString::from("JSON").into();
        let json = context.global_object().get(json_key, context)
            .map_err(|e| SuwayomiError::Extension(format!("Failed to get JSON object: {:?}", e)))?;
            
        let stringify_key: PropertyKey = JsString::from("stringify").into();
        let stringify = json.as_object().unwrap().get(stringify_key, context)
            .map_err(|e| SuwayomiError::Extension(format!("Failed to get JSON.stringify: {:?}", e)))?;
            
        let json_str_val = stringify.as_callable().unwrap()
            .call(&json, &[result], context)
            .map_err(|e| SuwayomiError::Extension(format!("JSON.stringify failed: {:?}", e)))?;
            
        let json_str = json_str_val.as_string()
            .ok_or_else(|| SuwayomiError::Extension("Result is not a string".to_string()))?
            .to_std_string_escaped();
            
        serde_json::from_str(&json_str)
            .map_err(|e| SuwayomiError::Extension(format!("Failed to deserialize MangaPage: {}", e)))
    }

    pub async fn load_script(&self, script: &str) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.sender.send(JsCommand::LoadScript(script.to_string(), tx))
            .map_err(|_| SuwayomiError::Internal("Runtime thread died".to_string()))?;
        rx.await.map_err(|_| SuwayomiError::Internal("Failed to receive response".to_string()))?
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
        self.runtime.sender.send(JsCommand::GetPopularManga(page, tx))
            .map_err(|_| SuwayomiError::Internal("Runtime thread died".to_string()))?;
        rx.await.map_err(|_| SuwayomiError::Internal("Failed to receive response".to_string()))?
    }

    async fn get_latest_updates(&self, _page: i32) -> Result<MangaPage> {
        let (tx, rx) = oneshot::channel();
        self.runtime.sender.send(JsCommand::GetLatestUpdates((), tx))
            .map_err(|_| SuwayomiError::Internal("Runtime thread died".to_string()))?;
        rx.await.map_err(|_| SuwayomiError::Internal("Failed to receive response".to_string()))?
    }

    async fn search_manga(&self, _query: &str, _filters: &[Filter], _page: i32) -> Result<MangaPage> {
        let (tx, rx) = oneshot::channel();
        self.runtime.sender.send(JsCommand::SearchManga((), (), (), tx))
            .map_err(|_| SuwayomiError::Internal("Runtime thread died".to_string()))?;
        rx.await.map_err(|_| SuwayomiError::Internal("Failed to receive response".to_string()))?
    }

    async fn get_manga_details(&self, _manga: Manga) -> Result<Manga> {
        let (tx, rx) = oneshot::channel();
        self.runtime.sender.send(JsCommand::GetMangaDetails((), tx))
            .map_err(|_| SuwayomiError::Internal("Runtime thread died".to_string()))?;
        rx.await.map_err(|_| SuwayomiError::Internal("Failed to receive response".to_string()))?
    }

    async fn get_chapter_list(&self, _manga: &Manga) -> Result<Vec<Chapter>> {
        let (tx, rx) = oneshot::channel();
        self.runtime.sender.send(JsCommand::GetChapterList((), tx))
            .map_err(|_| SuwayomiError::Internal("Runtime thread died".to_string()))?;
        rx.await.map_err(|_| SuwayomiError::Internal("Failed to receive response".to_string()))?
    }

    async fn get_page_list(&self, _chapter: &Chapter) -> Result<Vec<Page>> {
        let (tx, rx) = oneshot::channel();
        self.runtime.sender.send(JsCommand::GetPageList((), tx))
            .map_err(|_| SuwayomiError::Internal("Runtime thread died".to_string()))?;
        rx.await.map_err(|_| SuwayomiError::Internal("Failed to receive response".to_string()))?
    }

    async fn get_filters(&self) -> Result<Vec<Filter>> {
        let (tx, rx) = oneshot::channel();
        self.runtime.sender.send(JsCommand::GetFilters(tx))
            .map_err(|_| SuwayomiError::Internal("Runtime thread died".to_string()))?;
        rx.await.map_err(|_| SuwayomiError::Internal("Failed to receive response".to_string()))?
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
}
