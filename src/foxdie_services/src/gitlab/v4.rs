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

use crate::PushRequest;
use chrono::{DateTime, FixedOffset};
use glob::{Pattern, PatternError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct MergeRequestOptions {
    pub state_event: MergeRequestStateEvent,
}

#[derive(Debug, Copy, Clone, Serialize)]
pub enum MergeRequestStateEvent {
    #[serde(rename = "close")]
    Close,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MergeRequest {
    id: i32,
    iid: i32,
    project_id: i32,
    title: String,
    state: MergeRequestState,
    created_at: DateTime<FixedOffset>,
    updated_at: DateTime<FixedOffset>,
    target_branch: String,
    source_branch: String,
    author: Option<User>,
    source_project_id: i32,
    target_project_id: i32,
    web_url: String,
}

impl From<MergeRequest> for PushRequest {
    fn from(mr: MergeRequest) -> Self {
        PushRequest {
            url: mr.web_url,
            id: mr.iid,
            title: mr.title,
            created_at: mr.created_at,
            updated_at: mr.updated_at,
            target_project: mr.target_project_id,
            target_branch: mr.target_branch,
            source_project: mr.source_project_id,
            source_branch: mr.source_branch,
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub enum MergeRequestState {
    #[serde(rename = "opened")]
    Opened,
    #[serde(rename = "closed")]
    Closed,
    #[serde(rename = "locked")]
    Locked,
    #[serde(rename = "merged")]
    Merged,
}

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: Option<i32>,
    pub name: String,
    pub username: String,
    pub state: Option<UserState>,
    pub avatar_url: String,
    pub web_url: Option<String>,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub enum UserState {
    #[serde(rename = "all")]
    All,
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "inactive")]
    Inactive,
    #[serde(rename = "blocked")]
    Blocked,
    #[serde(rename = "ldap_blocked")]
    LdapBlocked,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProtectedBranch {
    pub name: String,
}

impl From<ProtectedBranch> for Result<crate::ProtectedBranch, PatternError> {
    fn from(branch: ProtectedBranch) -> Self {
        let pattern = Pattern::new(&branch.name)?;
        Ok(crate::ProtectedBranch { pattern })
    }
}
