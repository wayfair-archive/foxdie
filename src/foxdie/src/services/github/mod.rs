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

mod v3;

use std::convert::TryFrom;

pub(self) use self::v3::*;
use super::{PushRequest, PushRequestState, SCMProviderImpl};
use async_trait::async_trait;
use log::debug;
use reqwest::header;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use reqwest::Result as ReqwestResult;

#[derive(Debug)]
pub struct GitHub {
    client: Client,
    base_url: String,
    owner: String,
    repo: String,
}

impl GitHub {
    pub fn new(base_url: &str, token: &str, owner: &str, repo: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.append(
            header::ACCEPT,
            HeaderValue::from_static("application/vnd.github.v3+json"),
        );
        headers.append(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token))
                .expect("Token should be convertible to a header string"),
        );
        headers.append(
            header::USER_AGENT,
            HeaderValue::from_static("com.wayfair.foxdie"),
        );
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .expect("GitHub client failed to construct itself");
        GitHub {
            client,
            base_url: From::from(base_url),
            owner: From::from(owner),
            repo: From::from(repo),
        }
    }

    fn construct_base_url(&self) -> String {
        format!("{}/repos/{}/{}", self.base_url, self.owner, self.repo)
    }

    async fn paginated_request<Query, Intermediate, Output>(
        &self,
        url: &str,
        query: Query,
    ) -> ReqwestResult<Vec<Output>>
    where
        Query: serde::Serialize,
        Intermediate: serde::de::DeserializeOwned,
        Output: TryFrom<Intermediate>,
    {
        debug!("{}", url);

        let initial_resp = self.client.get(&*url).query(&query).send().await?;
        let mut headers = initial_resp.headers().clone();
        let page_items: Vec<Intermediate> = initial_resp.json().await?;

        let mut items: Vec<Output> = page_items
            .into_iter()
            .map(TryFrom::try_from)
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        while let Some(link_header) = headers.get(header::LINK).and_then(|h| h.to_str().ok()) {
            let links = Links::parse_from_rfc5988(link_header);
            if let Some(next) = links.next() {
                debug!("{}", next.uri);
                let resp = self.client.get(&*next.uri).send().await?;
                headers = resp.headers().clone();
                let mut push_requests =
                    resp.json().await.map(|page_items: Vec<Intermediate>| {
                        page_items
                            .into_iter()
                            .map(TryFrom::try_from)
                            .filter_map(Result::ok)
                            .collect::<Vec<_>>()
                    })?;
                items.append(&mut push_requests);
            } else {
                break;
            }
        }

        Ok(items)
    }
}

#[async_trait]
impl SCMProviderImpl for GitHub {
    async fn list_push_requests(&self, state: PushRequestState) -> ReqwestResult<Vec<PushRequest>> {
        self.paginated_request::<_, PullRequest, _>(
            &format!("{}/pulls", self.construct_base_url()),
            &[("state", state.github_value())],
        )
        .await
    }

    async fn close_push_request(&self, id: i32) -> ReqwestResult<()> {
        let url = format!("{}/pulls/{}", self.construct_base_url(), id);
        self.client
            .patch(&*url)
            .query(&PullRequestOptions {
                state: PullRequestStateEvent::Closed,
            })
            .send()
            .await
            .map(|_| ())
    }

    async fn list_protected_branches(&self) -> ReqwestResult<Vec<super::ProtectedBranch>> {
        self.paginated_request::<_, ProtectedBranch, _>(
            &format!("{}/branches", self.construct_base_url()),
            &[("protected", true)],
        )
        .await
    }
}

#[derive(Debug)]
struct Links {
    links: Vec<Link>,
}

#[allow(dead_code)]
impl Links {
    fn parse_from_rfc5988(header: &str) -> Self {
        Links {
            links: header
                .split(',')
                .map(Link::parse_from_rfc5988)
                .collect::<_>(),
        }
    }

    fn prev(&self) -> Option<&Link> {
        self.links.iter().find(|l| l.rel == "prev")
    }

    fn next(&self) -> Option<&Link> {
        self.links.iter().find(|l| l.rel == "next")
    }

    fn first(&self) -> Option<&Link> {
        self.links.iter().find(|l| l.rel == "first")
    }

    fn last(&self) -> Option<&Link> {
        self.links.iter().find(|l| l.rel == "last")
    }
}

#[derive(Debug)]
struct Link {
    uri: String,
    rel: String,
}

impl Link {
    fn parse_from_rfc5988(header: &str) -> Self {
        let mut components = header.split(';');
        let uri = components
            .next()
            .unwrap_or_default()
            .trim()
            .trim_start_matches('<')
            .trim_end_matches('>')
            .to_string();
        let rel = components
            .next()
            .unwrap_or_default()
            .trim()
            .trim_start_matches("rel=\"")
            .trim_end_matches('"')
            .to_string();
        Link { uri, rel }
    }
}
