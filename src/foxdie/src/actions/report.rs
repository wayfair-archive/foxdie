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
use crate::services::{git, PushRequest};
use chrono::{DateTime, TimeZone, Utc};
use log::info;
use serde::Serialize;
use serde_json;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn report<P>(output_path: &Option<P>, repo_path: Option<P>) -> Result<(), FoxdieError>
where
    P: AsRef<Path>,
{
    let repo = if let Some(p) = repo_path {
        git::open_repository(p)?
    } else {
        git::open_repository(env::current_dir().unwrap_or_default())?
    };
    let remotes = repo.remotes()?;
    let current_branch = git::get_current_branch(&repo)?;
    let push_requests = vec![];

    let mut reports = vec![];
    for remote_name in &remotes {
        let remote_name = if let Some(remote_name) = remote_name {
            remote_name
        } else {
            continue;
        };
        let mut remote = repo.find_remote(remote_name)?;
        git::fetch_refs(&mut remote)?;
        let report = report_for_remote(&repo, &remote, &current_branch, &push_requests)?;
        reports.push(report);
    }

    for report in reports {
        print_report(&report);
        if let Some(ref p) = output_path {
            write_report_to_disk(&report, p)?;
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct Report {
    remote_name: String,
    remote_url: String,
    items: Vec<ReportItem>,
}

#[derive(Debug, Serialize)]
struct ReportItem {
    upstream_diverged: usize,
    downstream_diverged: usize,
    branch: String,
    commit: String,
    author: String,
    last_updated: DateTime<Utc>,
    was_merge: bool,
    has_push_request: bool,
    message: String,
}

fn report_for_remote(
    repo: &git::Repository,
    remote: &git::Remote,
    current_branch: &git::Branch,
    push_requests: &[PushRequest],
) -> Result<Report, FoxdieError> {
    let branches = git::get_remote_branches(&repo)?
        .filter_map(Result::ok)
        .map(|pair| pair.0)
        .collect::<Vec<_>>();
    let source_branches = push_requests
        .iter()
        .map(|pr| pr.source_branch.to_string())
        .collect::<Vec<_>>();
    info!("Generating report for {} branches...", branches.len());
    let remote_name = remote.name().unwrap_or_default().to_string();
    let remote_url = remote.url().unwrap_or_default().to_string();
    let report_items = branches
        .iter()
        .filter_map(|branch| report_for_branch(repo, branch, current_branch, &source_branches))
        .collect::<Vec<_>>();
    Ok(Report {
        remote_name,
        remote_url,
        items: report_items,
    })
}

fn report_for_branch(
    repo: &git::Repository,
    branch: &git::Branch,
    current_branch: &git::Branch,
    push_request_branches: &[String],
) -> Option<ReportItem> {
    let branch_name = branch.name().ok()??;
    let commit = git::commit_for_branch(repo, branch).ok()?;
    let (upstream_diverged, downstream_diverged) =
        git::get_divergence_between_branches(repo, current_branch, branch).ok()?;
    let hash = commit.id().to_string();
    let author = commit.author().name()?.to_string();
    let last_updated = Utc.timestamp(commit.time().seconds(), 0);
    let has_push_request = push_request_branches.contains(&branch_name.to_string());
    let message = commit.message()?.to_string();
    Some(ReportItem {
        upstream_diverged,
        downstream_diverged,
        branch: branch_name.to_string(),
        commit: hash,
        author,
        last_updated,
        was_merge: false,
        has_push_request,
        message,
    })
}

fn print_report(report: &Report) {
    info!(
        "Report for {} ({})\n=================================",
        report.remote_name, report.remote_url
    );
    for item in &report.items {
        info!("{} – {}", item.author, item.branch);
    }
}

fn write_report_to_disk<P>(report: &Report, path: P) -> Result<(), FoxdieError>
where
    P: AsRef<Path>,
{
    let mut file = File::create(path)?;
    let json = serde_json::to_vec(report)?;
    file.write_all(&json).map_err(FoxdieError::from)
}
