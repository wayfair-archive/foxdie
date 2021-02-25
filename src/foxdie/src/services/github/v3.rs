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

use std::convert::TryFrom;

use super::PushRequest;
use chrono::{DateTime, FixedOffset};
use glob::{Pattern, PatternError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct PullRequestOptions {
    pub state: PullRequestStateEvent,
}

#[derive(Debug, Copy, Clone, Serialize)]
pub enum PullRequestStateEvent {
    #[serde(rename = "closed")]
    Closed,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PullRequest {
    pub id: i32,
    pub html_url: String,
    pub number: i32,
    pub title: String,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub head: GitData,
    pub base: GitData,
}

impl TryFrom<PullRequest> for PushRequest {
    type Error = ();

    fn try_from(pr: PullRequest) -> Result<Self, Self::Error> {
        Ok(PushRequest {
            url: pr.html_url,
            id: pr.number,
            title: pr.title,
            created_at: pr.created_at,
            updated_at: pr.updated_at,
            target_project: pr.base.repo.id,
            target_branch: pr.base.label,
            source_project: pr.head.repo.id,
            source_branch: pr.head.label,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitData {
    pub label: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub sha: String,
    pub repo: Repository,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Repository {
    pub id: i32,
    pub name: String,
    pub full_name: String,
    pub html_url: String,
    pub fork: bool,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub pushed_at: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProtectedBranch {
    pub name: String,
}

impl TryFrom<ProtectedBranch> for super::super::ProtectedBranch {
    type Error = PatternError;

    fn try_from(branch: ProtectedBranch) -> Result<Self, Self::Error> {
        let pattern = Pattern::new(&branch.name)?;
        Ok(Self { pattern })
    }
}
