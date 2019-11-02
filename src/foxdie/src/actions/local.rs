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
use crate::services::{
    get_api_client_for_remote, git, ProtectedBranch, PushRequest, PushRequestState,
};
use chrono::{DateTime, FixedOffset};
use log::{info, warn};
use std::env;
use std::path::Path;

pub struct Options<'a> {
    pub should_delete: bool,
    pub since_date: &'a DateTime<FixedOffset>,
    pub token: &'a str,
}

pub fn clean_remote_branches<P>(path: Option<P>, opts: Options) -> Result<(), FoxdieError>
where
    P: AsRef<Path>,
{
    let repo = if let Some(p) = path {
        git::open_repository(p)?
    } else {
        git::open_repository(env::current_dir().unwrap_or_default())?
    };
    let remotes = repo.remotes()?;
    for remote in remotes.into_iter().filter_map(|r| r) {
        clean_branches_on_remote(remote, &repo, &opts)?;
    }
    Ok(())
}

fn clean_branches_on_remote(
    remote_name: &str,
    repository: &git::Repository,
    opts: &Options,
) -> Result<(), FoxdieError> {
    let mut remote = repository.find_remote(remote_name)?;
    let api_client = if let Some(client) = get_api_client_for_remote(&remote, opts.token) {
        client
    } else {
        warn!(
            "{}",
            FoxdieError::UnsupportedProvider(remote.url().unwrap_or_default().to_string())
        );
        return Ok(());
    };

    git::fetch_refs(&mut remote)?;
    let current_local_branch = git::get_current_branch(&repository)?;
    let current_remote_branch = current_local_branch.upstream()?;

    let all_push_requests = api_client.list_push_requests(PushRequestState::Opened)?;
    let all_protected_branches = api_client.list_protected_branches()?;

    let all_branches = git::get_remote_branches(&repository)?.collect::<Vec<_>>();
    let all_branches_count = all_branches.len();

    let branches_to_delete = all_branches
        .into_iter()
        .filter_map(|res| res.ok().map(|pair| pair.0))
        .filter(is_branch_to_delete(
            remote_name,
            &current_remote_branch,
            opts.since_date,
            repository,
            &all_push_requests,
            &all_protected_branches,
        ))
        .collect::<Vec<_>>();

    print_branches_to_delete(&branches_to_delete, all_branches_count, remote_name);

    if !opts.should_delete {
        return Ok(());
    }

    delete_branches_if_needed(&branches_to_delete, repository, remote_name)
}

fn is_branch_to_delete<'a>(
    remote_name: &'a str,
    current_branch: &'a git::Branch,
    since_date: &'a DateTime<FixedOffset>,
    repository: &'a git::Repository,
    push_requests: &'a [PushRequest],
    protected_branches: &'a [ProtectedBranch],
) -> impl FnMut(&git::Branch<'a>) -> bool {
    move |branch| {
        branch.name().into_iter().flatten().any(|branch_name| {
            let branch_name = removing_remote_from_tracking_branch(branch_name, remote_name);
            branch.get() != current_branch.get()
                && !git::has_branch_updated_since(&repository, &branch, since_date).unwrap_or(true)
                && !push_requests
                    .iter()
                    .any(|pr| pr.source_branch == branch_name)
                && !protected_branches
                    .iter()
                    .any(|branch| branch.matches_branch(&branch_name))
        })
    }
}

fn removing_remote_from_tracking_branch(branch_name: &str, remote_name: &str) -> String {
    let tracking_prefix = &*format!("{}/", remote_name);
    branch_name.replace(tracking_prefix, "")
}

fn print_branches_to_delete(
    branches: &[git::Branch],
    all_branches_count: usize,
    remote_name: &str,
) {
    info!(
        "Found {} eligible branches out of {} total on {}{}",
        branches.len(),
        all_branches_count,
        remote_name,
        if !branches.is_empty() {
            let branches_message = branches
                .iter()
                .filter_map(|branch| branch.name().ok())
                .flatten()
                .map(|name| format!("• {}\n", name))
                .collect::<String>();
            format!(":\n{}", branches_message)
        } else {
            ".".to_string()
        }
    );
}

fn delete_branches_if_needed(
    branches: &[git::Branch],
    repository: &git::Repository,
    remote_name: &str,
) -> Result<(), FoxdieError> {
    info!("Preparing to delete {} branches...", branches.len());

    let refspecs = branches
        .iter()
        .filter_map(|branch| branch.name().ok())
        .flatten()
        .map(|branch_name| {
            format!(
                "+:refs/heads/{}",
                removing_remote_from_tracking_branch(branch_name, remote_name)
            )
        })
        .collect::<Vec<_>>();

    let refspecs_slice = refspecs.iter().map(|spec| &**spec).collect::<Vec<_>>();
    git::push_to_remote(&repository, remote_name, &refspecs_slice).map_err(FoxdieError::from)?;

    info!("Finished deleting branches.");
    Ok(())
}
