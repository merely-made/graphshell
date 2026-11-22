// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;

pub(super) fn build_runtime() -> Result<Runtime, String> {
    Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|_| "could not build fetch runtime".into())
}

pub(super) fn run_jobs(
    runtime: &Runtime,
    shared: &Arc<Shared>,
    mut next: impl FnMut() -> Option<Job>,
    stores: Stores,
    policy: ClientFetchPolicy,
) {
    let tasks = ResidentTasks::new();
    while let Some(mut job) = next() {
        if shared.is_closed() {
            drop(job);
        } else {
            let _entered = runtime.enter();
            job.spawned = true;
            tasks.spawn(job.run(stores.clone(), policy));
        }
    }
    shared.close();
    let tasks = runtime.block_on(tasks.close_and_join());
    *shared.report.lock().expect("fetch report mutex poisoned") = Some(FetchActorReport {
        tasks,
        admitted: shared.admitted.load(Ordering::Relaxed),
        discarded_before_spawn: shared.discarded_before_spawn.load(Ordering::Relaxed),
        overloaded: shared.overloaded.load(Ordering::Relaxed),
        queue_timeouts: shared.queue_timeouts.load(Ordering::Relaxed),
    });
}

pub(super) async fn execute(
    command: FetchCommand,
    stores: Stores,
    policy: ClientFetchPolicy,
    hop_admission: Option<&SmolwebHopAdmission>,
    completion: Arc<Completion>,
) -> FetchUpdate {
    match command {
        FetchCommand::Page {
            request,
            url,
            identity,
        } => {
            #[cfg(feature = "peer-editions")]
            if crate::peer::is_address(&url) {
                let result = match tokio::time::timeout(policy.timeout, crate::peer::load(&url, policy)).await {
                    Ok(Ok((page, assets))) => {
                        if !completion.terminal.load(Ordering::Acquire) {
                            let response_url = assets.document_url.clone();
                            completion.out.emit(FetchUpdate::PeerAssets { request, url: url.clone(), assets: Arc::new(assets) });
                            completion.out.emit(FetchUpdate::PageProgress(PageProgress {
                                request, url: url.clone(), response_url, content_type: page.content_type.clone(), bytes: page.bytes.clone(),
                            }));
                        }
                        Ok(page)
                    }
                    Ok(Err(error)) => Err(FetchFailure::Failed(error)),
                    Err(_) => Err(FetchFailure::Failed("IPFS page timed out".into())),
                };
                return FetchUpdate::Page(FetchOutcome { request, url, result });
            }
            let request_url = url.clone();
            let progress = completion.clone();
            let result = fetch_page_interactive_capped(
                &url,
                policy.max_page_bytes,
                identity.as_ref(),
                &stores,
                policy,
                hop_admission,
                move |response_url, content_type, bytes| {
                    if !progress.terminal.load(Ordering::Acquire) {
                        progress.out.emit(FetchUpdate::PageProgress(PageProgress {
                            request,
                            url: request_url.clone(),
                            response_url: response_url.to_string(),
                            content_type: content_type.map(str::to_string),
                            bytes: bytes.to_vec(),
                        }));
                    }
                },
            )
            .await;
            FetchUpdate::Page(FetchOutcome {
                request,
                url,
                result,
            })
        },
        FetchCommand::Subresource(url) => {
            let result = fetch_bytes(&url, &stores, policy, hop_admission).await;
            FetchUpdate::Subresource(SubresourceOutcome { url, result })
        },
        FetchCommand::Favicon { request, url } => FetchUpdate::Favicon(FaviconOutcome {
            request,
            result: fetch_bytes(&url, &stores, policy, hop_admission).await,
        }),
        FetchCommand::Submit {
            request,
            submission,
        } => {
            let url = submission.url().into();
            #[cfg(feature = "smolweb")]
            let result = submit_smolweb(submission, policy, hop_admission).await;
            #[cfg(not(feature = "smolweb"))]
            let result = Err(FetchFailure::Failed(
                "small-web submissions are not built into this host".into(),
            ));
            FetchUpdate::Submission(SubmissionOutcome {
                request,
                url,
                result,
            })
        },
        FetchCommand::CancelPage { .. } => unreachable!("controls bypass work admission"),
    }
}
