// Copyright (c) 2018-2019, Wayfair LLC
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
// following conditions are met:
//
//  * Redistributions of source code must retain the above copyright notice, this list of conditions and the following
//    disclaimer.
//  * Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the
//    following disclaimer in the documentation and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING,
// BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
// IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY,
// OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
// DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT,
// STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE,
// EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use crate::error::FoxdieError;
use crate::services::{get_api_client_for_url, PushRequest, PushRequestState};
use chrono::{DateTime, FixedOffset};
use log::info;

pub async fn clean_push_requests(
    should_delete: bool,
    since_date: &DateTime<FixedOffset>,
    url: &str,
    token: &str,
) -> Result<(), FoxdieError> {
    let api_client = if let Some(client) = get_api_client_for_url(url, token).await {
        client
    } else {
        return Err(FoxdieError::UnsupportedProvider(url.to_string()));
    };
    info!(
        "Checking for push requests created from before {:?}.",
        since_date
    );
    let all_push_requests = api_client
        .list_push_requests(PushRequestState::Opened)
        .await?;
    let all_push_requests_count = all_push_requests.len();
    let eligible_push_requests = all_push_requests
        .into_iter()
        .filter(|pr| pr.target_project == pr.source_project && pr.updated_at < *since_date)
        .collect::<Vec<_>>();

    print_push_requests_to_close(&eligible_push_requests, all_push_requests_count);

    if !should_delete {
        return Ok(());
    }
    info!("\nPreparing to close push requests...");
    for pr in &eligible_push_requests {
        api_client.close_push_request(pr.id).await?;
        info!("Closed #{}", pr.id);
    }
    info!("All done closing push requests.");
    Ok(())
}

fn print_push_requests_to_close(push_requests: &[PushRequest], all_push_requests_count: usize) {
    info!(
        "Found {} eligible push requests out of {} total{}",
        push_requests.len(),
        all_push_requests_count,
        if !push_requests.is_empty() {
            let push_requests_message = push_requests
                .iter()
                .map(|pr| format!("• #{}: {} ({})\n", pr.id, pr.title, pr.url))
                .collect::<String>();
            format!(":\n{}", push_requests_message)
        } else {
            String::from("")
        }
    );
}
