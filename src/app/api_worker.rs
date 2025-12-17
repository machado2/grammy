use std::sync::mpsc::{Receiver, Sender};

use crate::api;
use crate::config::ApiProvider;
use crate::suggestion::Suggestion;

#[derive(Debug)]
pub(super) enum ApiJob {
    Grammar {
        text: String,
        api_key: String,
        model: String,
        provider: ApiProvider,
    },
    TestConnection {
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
                    } => match api::check_grammar(text, api_key, model, provider, request_id).await {
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
                    ApiJob::TestConnection { api_key, provider } => {
                        match api::test_connection(api_key, provider, request_id).await {
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
                    }
                }
            });
        }

        eprintln!("[DEBUG] API thread exiting");
    });
}
