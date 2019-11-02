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

//! `foxdie_services` contains modules pertaining to integrations. Currently, interfaces to Gitlab, GitHub, and Git are
//! all located here.

pub mod git;
mod github;
mod gitlab;

use self::git::Remote;
use self::github::GitHub;
use self::gitlab::Gitlab;
use chrono::{DateTime, FixedOffset};
use glob::Pattern;
use log::error;
use reqwest::Result as ReqwestResult;
use what_git::{SCMKind, SCM};

/// Return `Some(SCMProvider)` if the given Git remote can be associated with a known and supported Git SCM. Otherwise,
/// print an error and return `None`.
pub fn get_api_client_for_remote(remote: &Remote, token: &str) -> Option<SCMProvider> {
    if let Some(url) = remote.url() {
        get_api_client_for_url(url, token)
    } else {
        None
    }
}

/// Return `Some(SCMProvider)` if the given Git remote URL can be associated with a known and supported Git SCM.
/// Otherwise, print an error and return `None`.
pub fn get_api_client_for_url(url: &str, token: &str) -> Option<SCMProvider> {
    match what_git::what_git(url, token) {
        Ok(description) => SCMProvider::from_scm_description(description, token),
        Err(err) => {
            error!("{}", err);
            None
        }
    }
}

pub(crate) trait SCMProviderImpl {
    fn list_push_requests(&self, state: PushRequestState) -> ReqwestResult<Vec<PushRequest>>;
    fn close_push_request(&self, id: i32) -> ReqwestResult<()>;
    fn list_protected_branches(&self) -> ReqwestResult<Vec<ProtectedBranch>>;
}

/// Wrapper for an `SCMProviderImpl` implementer. Bridges generic SCM API requests to the appropriate platform type.
pub struct SCMProvider {
    inner: Box<dyn SCMProviderImpl>,
}

impl SCMProvider {
    fn from_scm_description(description: SCM, token: &str) -> Option<Self> {
        match description {
            SCM {
                kind: SCMKind::GitHub,
                ..
            } => Some(SCMProvider {
                inner: Box::new(GitHub::new(
                    &description.base_url,
                    token,
                    &description.owner,
                    &description.repo,
                )),
            }),
            SCM {
                kind: SCMKind::Gitlab,
                ..
            } => Some(SCMProvider {
                inner: Box::new(Gitlab::new(
                    &description.base_url,
                    token,
                    &description.owner,
                    &description.repo,
                )),
            }),
            _ => None,
        }
    }

    pub fn list_push_requests(&self, state: PushRequestState) -> ReqwestResult<Vec<PushRequest>> {
        self.inner.list_push_requests(state)
    }

    pub fn close_push_request(&self, id: i32) -> ReqwestResult<()> {
        self.inner.close_push_request(id)
    }

    pub fn list_protected_branches(&self) -> ReqwestResult<Vec<ProtectedBranch>> {
        self.inner.list_protected_branches()
    }
}

#[derive(Debug)]
pub enum PushRequestState {
    Opened,
    #[allow(dead_code)]
    Closed,
}

impl PushRequestState {
    fn github_value(&self) -> &'static str {
        match self {
            PushRequestState::Opened => "open",
            PushRequestState::Closed => "closed",
        }
    }

    fn gitlab_value(&self) -> &'static str {
        match self {
            PushRequestState::Opened => "opened",
            PushRequestState::Closed => "closed",
        }
    }
}

#[derive(Debug)]
pub struct PushRequest {
    pub url: String,
    pub id: i32,
    pub title: String,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub target_project: i32,
    pub target_branch: String,
    pub source_project: i32,
    pub source_branch: String,
}

#[derive(Debug)]
pub struct ProtectedBranch {
    pub pattern: Pattern,
}

impl ProtectedBranch {
    /// Given this branch's pattern string
    pub fn matches_branch(&self, branch: &str) -> bool {
        self.pattern.matches(branch)
    }
}
