use std::sync::mpsc::{Receiver, Sender};

use crate::api;
use crate::config::ApiProvider;
use crate::suggestion::Suggestion;

use super::history::HistoryEntry;

#[derive(Debug)]
pub(super) enum ApiJob {
    Grammar {
        text: String,
        api_key: String,
        model: String,
        provider: ApiProvider,
        history: Vec<HistoryEntry>,
    },
    TestConnection {
        api_key: String,
        provider: ApiProvider,
        model: String,
    },
    FetchModels {
        api_key: String,
        provider: ApiProvider,
    },
}

#[derive(Debug)]
pub(super) struct ApiRequest {
    pub(super) job: ApiJob,
    pub(super) request_id: u64,
}

#[derive(Debug, Clone)]
pub(super) enum ApiResponse {
    GrammarSuccess {
        suggestions: Vec<Suggestion>,
        request_id: u64,
    },
    GrammarError {
        message: String,
        request_id: u64,
    },
    TestSuccess {
        request_id: u64,
    },
    TestError {
        message: String,
        request_id: u64,
    },
    ModelsSuccess {
        models: Vec<String>,
        provider: ApiProvider,
    },
    ModelsError {
        message: String,
    },
}

pub(super) fn spawn_api_worker(request_rx: Receiver<ApiRequest>, response_tx: Sender<ApiResponse>) {
    std::thread::spawn(move || {
        eprintln!("[DEBUG] API thread started");
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");

        while let Ok(req) = request_rx.recv() {
            eprintln!("[DEBUG] API thread received request #{}", req.request_id);
            let tx = response_tx.clone();
            let request_id = req.request_id;

            rt.block_on(async {
                match req.job {
                    ApiJob::Grammar {
                        text,
                        api_key,
                        model,
                        provider,
                        history,
                    } => match api::check_grammar(
                        text, api_key, model, provider, request_id, history,
                    )
                    .await
                    {
                        Ok((suggestions, req_id)) => {
                            let _ = tx.send(ApiResponse::GrammarSuccess {
                                suggestions,
                                request_id: req_id,
                            });
                        }
                        Err(e) => {
                            let _ = tx.send(ApiResponse::GrammarError {
                                message: e,
                                request_id,
                            });
                        }
                    },
                    ApiJob::TestConnection {
                        api_key,
                        provider,
                        model,
                    } => match api::test_connection(api_key, provider, model, request_id).await {
                        Ok(req_id) => {
                            let _ = tx.send(ApiResponse::TestSuccess { request_id: req_id });
                        }
                        Err(e) => {
                            let _ = tx.send(ApiResponse::TestError {
                                message: e,
                                request_id,
                            });
                        }
                    },
                    ApiJob::FetchModels { api_key, provider } => {
                        let provider_clone = provider.clone();
                        match api::fetch_models(provider, api_key).await {
                            Ok(models) => {
                                let _ = tx.send(ApiResponse::ModelsSuccess {
                                    models,
                                    provider: provider_clone,
                                });
                            }
                            Err(e) => {
                                let _ = tx.send(ApiResponse::ModelsError { message: e });
                            }
                        }
                    }
                }
            });
        }

        eprintln!("[DEBUG] API thread exiting");
    });
}
