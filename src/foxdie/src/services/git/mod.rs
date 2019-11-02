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

use chrono::{DateTime, FixedOffset};
use git2::{self, Branches, Commit};
use log::{debug, info};
use std::path::Path;

pub use git2::{Branch, Error, Remote, Repository};

pub fn open_repository<P>(path: P) -> Result<Repository, Error>
where
    P: AsRef<Path>,
{
    Repository::open(path)
}

fn authorized_remote_callbacks<'a>() -> Result<git2::RemoteCallbacks<'a>, Error> {
    let config = git2::Config::open_default()?;
    let mut cbs = git2::RemoteCallbacks::new();
    cbs.credentials(move |url, username_from_url, allowed_types| {
        if allowed_types.contains(git2::CredentialType::SSH_KEY)
            || allowed_types.contains(git2::CredentialType::SSH_CUSTOM)
        {
            let username = username_from_url
                .expect("A username in the URL is required for SSH and Git to work.");
            git2::Cred::ssh_key_from_agent(username)
        } else {
            git2::Cred::credential_helper(&config, url, username_from_url)
        }
    });
    cbs.sideband_progress(|data| {
        use std::io::{self, Write};
        use std::str;

        debug!(
            "remote: {}",
            str::from_utf8(data).unwrap_or("err: unable to convert data to utf8 string\n")
        );
        io::stdout()
            .flush()
            .expect("could not flush libgit IO stream");
        true
    });
    Ok(cbs)
}

pub fn fetch_refs(remote: &mut Remote) -> Result<(), Error> {
    let mut opts = git2::FetchOptions::new();
    opts.remote_callbacks(authorized_remote_callbacks()?);
    info!(
        "Fetching remote refs from {} ({})",
        remote.name().unwrap_or("[UNKNOWN REMOTE NAME]"),
        remote.url().unwrap_or("[UNKNOWN REMOTE URL]")
    );
    remote.fetch(&[], Some(&mut opts), None)
}

pub fn get_current_branch(repo: &Repository) -> Result<Branch, Error> {
    let head = repo.head()?;
    if head.is_branch() {
        Ok(Branch::wrap(head))
    } else {
        Err(Error::from_str("HEAD must be a branch"))
    }
}

pub fn get_remote_branches(repo: &Repository) -> Result<Branches, Error> {
    repo.branches(Some(git2::BranchType::Remote))
}

pub fn get_divergence_between_branches(
    repo: &Repository,
    left: &Branch,
    right: &Branch,
) -> Result<(usize, usize), Error> {
    let left_oid = branch_to_oid(&left)?;
    let right_oid = branch_to_oid(&right)?;
    repo.graph_ahead_behind(left_oid, right_oid)
}

pub fn has_branch_updated_since(
    repo: &Repository,
    branch: &Branch,
    date: &DateTime<FixedOffset>,
) -> Result<bool, Error> {
    let commit = commit_for_branch(repo, branch)?;
    let git_time = commit.time();
    let timestamp = git_time.seconds();
    Ok(timestamp > date.timestamp())
}

pub fn commit_for_branch<'repo>(
    repo: &'repo git2::Repository,
    branch: &Branch,
) -> Result<Commit<'repo>, Error> {
    let oid = branch_to_oid(branch)?;
    repo.find_commit(oid)
}

fn branch_to_oid(branch: &Branch) -> Result<git2::Oid, Error> {
    branch
        .get()
        .resolve()?
        .target()
        .ok_or_else(|| Error::from_str("Could not peel OID from branch"))
}

pub fn push_to_remote(repo: &Repository, remote: &str, refspecs: &[&str]) -> Result<(), Error> {
    let mut remote = repo.find_remote(remote)?;
    let mut opts = git2::PushOptions::new();
    opts.remote_callbacks(authorized_remote_callbacks()?);
    remote.push(refspecs, Some(&mut opts))
}
