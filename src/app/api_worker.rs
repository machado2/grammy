use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};

use crate::api;
use crate::config::{ApiProvider, ReasoningEffort};
use crate::suggestion::Suggestion;

use super::history::HistoryEntry;

#[derive(Debug)]
pub(super) enum ApiJob {
    Grammar {
        text: String,
        api_key: String,
        model: String,
        provider: ApiProvider,
        reasoning_effort: ReasoningEffort,
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
    CancelGrammar,
    CancelTestConnection,
    CancelModels {
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
        request_id: u64,
    },
    ModelsError {
        message: String,
        provider: ApiProvider,
        request_id: u64,
    },
}

pub(super) fn spawn_api_worker(request_rx: Receiver<ApiRequest>, response_tx: Sender<ApiResponse>) {
    std::thread::spawn(move || {
        eprintln!("[DEBUG] API thread started");
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        let mut grammar_task: Option<(u64, tokio::task::JoinHandle<()>)> = None;
        let mut test_task: Option<(u64, tokio::task::JoinHandle<()>)> = None;
        let mut models_tasks: HashMap<ApiProvider, (u64, tokio::task::JoinHandle<()>)> =
            HashMap::new();

        while let Ok(req) = request_rx.recv() {
            eprintln!("[DEBUG] API thread received request #{}", req.request_id);
            let request_id = req.request_id;

            match req.job {
                ApiJob::Grammar {
                    text,
                    api_key,
                    model,
                    provider,
                    reasoning_effort,
                    history,
                } => {
                    if let Some((prev_id, prev)) = grammar_task.take() {
                        if !prev.is_finished() {
                            eprintln!("[DEBUG] Cancelling grammar request #{}", prev_id);
                            prev.abort();
                        }
                    }

                    let tx = response_tx.clone();
                    let client = client.clone();
                    let handle = rt.spawn(async move {
                        match api::check_grammar(
                            &client,
                            text,
                            api_key,
                            model,
                            provider,
                            reasoning_effort,
                            request_id,
                            history,
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
                        }
                    });
                    grammar_task = Some((request_id, handle));
                }
                ApiJob::TestConnection {
                    api_key,
                    provider,
                    model,
                } => {
                    if let Some((prev_id, prev)) = test_task.take() {
                        if !prev.is_finished() {
                            eprintln!("[DEBUG] Cancelling test request #{}", prev_id);
                            prev.abort();
                        }
                    }

                    let tx = response_tx.clone();
                    let client = client.clone();
                    let handle = rt.spawn(async move {
                        match api::test_connection(&client, api_key, provider, model, request_id)
                            .await
                        {
                            Ok(req_id) => {
                                let _ = tx.send(ApiResponse::TestSuccess { request_id: req_id });
                            }
                            Err(e) => {
                                let _ = tx.send(ApiResponse::TestError {
                                    message: e,
                                    request_id,
                                });
                            }
                        }
                    });
                    test_task = Some((request_id, handle));
                }
                ApiJob::FetchModels { api_key, provider } => {
                    if let Some((prev_id, prev)) = models_tasks.remove(&provider) {
                        if !prev.is_finished() {
                            eprintln!(
                                "[DEBUG] Cancelling models fetch request #{} (provider={})",
                                prev_id,
                                provider.name()
                            );
                            prev.abort();
                        }
                    }

                    let tx = response_tx.clone();
                    let client = client.clone();
                    let provider_key = provider.clone();
                    let handle = rt.spawn(async move {
                        let provider_for_response = provider.clone();
                        match api::fetch_models(&client, provider, api_key).await {
                            Ok(models) => {
                                let _ = tx.send(ApiResponse::ModelsSuccess {
                                    models,
                                    provider: provider_for_response,
                                    request_id,
                                });
                            }
                            Err(e) => {
                                let _ = tx.send(ApiResponse::ModelsError {
                                    message: e,
                                    provider: provider_for_response,
                                    request_id,
                                });
                            }
                        }
                    });
                    models_tasks.insert(provider_key, (request_id, handle));
                }
                ApiJob::CancelGrammar => {
                    if let Some((prev_id, prev)) = grammar_task.take() {
                        if !prev.is_finished() {
                            eprintln!("[DEBUG] Cancelling grammar request #{}", prev_id);
                            prev.abort();
                        }
                    }
                }
                ApiJob::CancelTestConnection => {
                    if let Some((prev_id, prev)) = test_task.take() {
                        if !prev.is_finished() {
                            eprintln!("[DEBUG] Cancelling test request #{}", prev_id);
                            prev.abort();
                        }
                    }
                }
                ApiJob::CancelModels { provider } => {
                    if let Some((prev_id, prev)) = models_tasks.remove(&provider) {
                        if !prev.is_finished() {
                            eprintln!(
                                "[DEBUG] Cancelling models fetch request #{} (provider={})",
                                prev_id,
                                provider.name()
                            );
                            prev.abort();
                        }
                    }
                }
            }
        }

        eprintln!("[DEBUG] API thread exiting");
    });
}
