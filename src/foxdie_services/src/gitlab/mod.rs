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

mod v4;

pub(self) use self::v4::*;
use crate::{PushRequest, SCMProviderImpl};
use log::{debug, error};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use reqwest::Result as ReqwestResult;
use url::percent_encoding::{utf8_percent_encode, PATH_SEGMENT_ENCODE_SET};

#[derive(Debug)]
pub struct Gitlab {
    client: Client,
    base_url: String,
    owner: String,
    repo: String,
}

impl Gitlab {
    pub fn new(base_url: &str, token: &str, owner: &str, repo: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.append(
            "private-token",
            HeaderValue::from_str(token).expect("Token should be convertible to a header string"),
        );
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .expect("Gitlab client failed to construct itself.");
        Gitlab {
            client,
            base_url: From::from(base_url),
            owner: From::from(owner),
            repo: From::from(repo),
        }
    }

    fn construct_base_url(&self) -> String {
        let namespace = format!("{}/{}", self.owner, self.repo);
        let namespace_encoded = utf8_percent_encode(&namespace[..], PATH_SEGMENT_ENCODE_SET);
        format!("{}/api/v4/projects/{}", self.base_url, namespace_encoded)
    }

    fn merge_requests_for_page(
        &self,
        state: &'static str,
        page: &str,
    ) -> ReqwestResult<Vec<MergeRequest>> {
        let url = format!("{}/merge_requests", self.construct_base_url());
        debug!("{}", url);
        self.client
            .get(&*url)
            .query(&[("state", state), ("page", page)])
            .send()?
            .json()
    }

    fn handle_error(error: reqwest::Error) -> reqwest::Error {
        if error.is_http() {
            match error.url() {
                None => error!("No URL provided"),
                Some(url) => error!("Problem making request to: {}", url),
            }
        }
        if error.is_serialization() {
            let serde_error = match error.get_ref() {
                Some(err) => err,
                _ => return error,
            };
            error!("Serialization error: {}", serde_error);
        }
        if error.is_redirect() {
            error!("Server redirection error");
        }
        error
    }
}

impl SCMProviderImpl for Gitlab {
    fn list_push_requests(&self, state: &'static str) -> ReqwestResult<Vec<PushRequest>> {
        let url = format!("{}/merge_requests", self.construct_base_url());
        debug!("{}", url);
        let query = [("state", state)];

        let head = self
            .client
            .head(&*url)
            .query(&query)
            .send()
            .map_err(Gitlab::handle_error)?;
        let headers = head.headers();
        let pages = Pages::new(&headers);

        if let Pages {
            current: Some(current),
            total_items: Some(total_items),
            total_pages: Some(total_pages),
            ..
        } = pages
        {
            let mut items = Vec::with_capacity(total_items);
            for page in current..=total_pages {
                let mut push_requests = self
                    .merge_requests_for_page(state, &*page.to_string())
                    .map(|merge_requests| {
                        merge_requests
                            .into_iter()
                            .map(From::from)
                            .collect::<Vec<_>>()
                    })
                    .map_err(Gitlab::handle_error)?;
                items.append(&mut push_requests);
            }
            Ok(items)
        } else {
            Ok(vec![])
        }
    }

    fn close_push_request(&self, id: i32) -> ReqwestResult<()> {
        let url = format!("{}/merge_requests/{}", self.construct_base_url(), id);
        self.client
            .put(&*url)
            .query(&MergeRequestOptions {
                state_event: MergeRequestStateEvent::Close,
            })
            .send()
            .map(|_res| ())
            .map_err(Gitlab::handle_error)
    }

    fn list_protected_branches(&self) -> ReqwestResult<Vec<crate::ProtectedBranch>> {
        let url = format!("{}/protected_branches", self.construct_base_url());
        let protected_branches: Vec<ProtectedBranch> = self.client.get(&*url).send()?.json()?;
        Ok(protected_branches
            .into_iter()
            .map(From::from)
            .filter_map(Result::ok)
            .collect())
    }
}

#[allow(dead_code)]
#[derive(Debug)]
struct Pages {
    current: Option<usize>,
    total_items: Option<usize>,
    total_pages: Option<usize>,
    per_page: Option<usize>,
    previous: Option<usize>,
    next: Option<usize>,
}

impl Pages {
    fn new(headers: &HeaderMap) -> Self {
        Pages {
            current: Pages::x_header(&headers, "x-page"),
            total_items: Pages::x_header(&headers, "x-total"),
            total_pages: Pages::x_header(&headers, "x-total-pages"),
            per_page: Pages::x_header(&headers, "x-per-page"),
            previous: Pages::x_header(&headers, "x-prev-page"),
            next: Pages::x_header(&headers, "x-next-page"),
        }
    }

    fn x_header(headers: &HeaderMap, key: &'static str) -> Option<usize> {
        headers
            .get(key)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.parse::<usize>().ok())
    }
}
