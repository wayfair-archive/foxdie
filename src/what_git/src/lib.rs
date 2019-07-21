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

//! `what_git` provides an easy mechanism for associating a given Git repository URL with its source. It supports
//! either GitHub, GitHub Enterprise, Gitlab, or Gitlab Enterprise repositories. Use this crate to structure
//! SCM-agnostic code with minimal branching.
//!
//! # About
//!
//! `what_git` associates a repository URL to a known repository source. All you need is an HTTP or Git URL,
//! and a personal access token to the API service your repository is associated with. Provide each of those to the
//! [`what_git::what_git`] function, and that's it.
//!
//!
//! [`what_git::what_git`]: ./fn.what_git.html

use reqwest::header;
use reqwest::{Client, Url};
use std::env;
use std::error;
use std::fmt;
use std::result;

/// Determines what source control management (SCM) solution a repository URL belongs to. Returns a
/// [`what_git::Result`] type describing the structure of the URL and the associated [`what_git::SCMKind`], or some
/// error of type [`what_git::Error`].
///
/// # Examples
///
/// ```no_run
/// use what_git::{what_git, SCMKind};
///
/// fn main() {
///     let scm = what_git("https://github.com/rust-lang/rust", "<PERSONAL ACCESS TOKEN>").unwrap();
///     if scm.kind == SCMKind::GitHub {
///         println!("Do something with GitHub...");
///     }
/// }
/// ```
/// [`what_git::Result`]: ./type.Result.html
/// [`what_git::SCMKind`]: ./enum.SCMKind.html
/// [`what_git::Error`]: ./enum.Error.html
pub fn what_git(repository: &str, token: &str) -> Result {
    let url_str = scrub_git_url_if_needed(repository);
    let url = Url::parse(&url_str).or_else(|_| Err(Error::UnknownProvider(url_str.to_string())))?;
    metadata_for_url(&url, token)
}

/// Remove various non-standard decorations, such as SSH decorations, from a URL string to get a string conforming to
/// the [URL Standard](http://url.spec.whatwg.org/).
fn scrub_git_url_if_needed(repository: &str) -> String {
    if repository.starts_with("git@") {
        repository
            .replacen(":", "/", 1)
            .replacen("git@", "git://", 1)
    } else {
        repository.to_string()
    }
}

/// Determines what source control management (SCM) solution a repository URL belongs to. Returns a [`what_git::Result`]
/// type describing the structure of the URL and the associated [`what_git::SCMKind`], or some error of type
/// [`what_git::Error`].
fn metadata_for_url(url: &Url, token: &str) -> Result {
    // Extract the first two path components in the URL to guess at the repository owner and name.
    let path_components = url
        .path_segments()
        .expect(
            "URL path components could not be represented.
    This is likely because it is not a valid URL for this tool.",
        )
        .take(2)
        .collect::<Vec<&str>>();
    let (owner, mut repo) = if let [own, rep] = path_components[..] {
        (own, rep)
    } else {
        return Err(Error::UnknownProvider(url.to_string()));
    };
    if let Some(idx) = repo.rfind(".git") {
        repo = &repo[..idx];
    }

    // Extract the hostname
    let hostname = url
        .domain()
        .ok_or_else(|| Error::UnknownProvider(url.to_string()))?;

    let base_url: String;
    let kind: SCMKind;

    if hostname == "github.com" || hostname == "www.github.com" {
        // 1. If the repository is located on GitHub.com, proceed
        base_url = "https://api.github.com".to_string();
        kind = SCMKind::GitHub;
    } else if hostname == "gitlab.com" || hostname == "www.gitlab.com" {
        // 2. If the repository is located on Gitlab.com, proceed
        base_url = "https://gitlab.com".to_string();
        kind = SCMKind::Gitlab;
    } else if let Ok(base) = env::var("GITHUB_BASE_URL") {
        // 3. If the user has manually specified an API base URL for a GitHub repository, proceed
        base_url = base;
        kind = SCMKind::GitHub;
    } else if let Ok(base) = env::var("GITLAB_BASE_URL") {
        // 4. If the user has manually specified an API base URL for a Gitlab repository, proceed
        base_url = base;
        kind = SCMKind::GitHub;
    } else {
        // 5. Attempt to connect to an SCM's API using known unique endpoints, and match on the possible successes.
        let base_url_candidate = format!("https://{}", hostname);
        match (
            verify_github(&base_url_candidate, token),
            verify_gitlab(&base_url_candidate, token),
        ) {
            (Ok(true), _) => {
                base_url = base_url_candidate;
                kind = SCMKind::GitHub;
            }
            (_, Ok(true)) => {
                base_url = base_url_candidate;
                kind = SCMKind::Gitlab;
            }
            _ => return Err(Error::UnknownProvider(url.to_string())),
        };
    }
    Ok(SCM {
        base_url,
        kind,
        owner: owner.to_string(),
        repo: repo.to_string(),
    })
}

// Attempt to connect to the GitHub `/zen` endpoint, which is unique to GitHub's API.
fn verify_github(base_url: &str, token: &str) -> result::Result<bool, reqwest::Error> {
    let url = format!("{}/zen", base_url);

    Client::new()
        .get(&*url)
        .header(header::ACCEPT, "application/vnd.github.v3+json")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(header::USER_AGENT, "com.wayfair.what_gitjson")
        .send()
        .map(|res| res.status().is_success())
}

// Attempt to connect to the Gitlab `/version` endpoint, which is unique to Gitlab's API.
fn verify_gitlab(base_url: &str, token: &str) -> result::Result<bool, reqwest::Error> {
    let url = format!("{}/api/v4/version", base_url);

    Client::new()
        .get(&*url)
        .header("private-token", token)
        .send()
        .map(|res| res.status().is_success())
}

/// Used to describe the structure of a repository on a supported source control management (SCM) solution.
#[derive(Debug)]
pub struct SCM {
    pub kind: SCMKind,
    /// The base URL used in API calls
    pub base_url: String,
    /// The user or organization space that owns the repository
    pub owner: String,
    /// The name of the repository
    pub repo: String,
}

/// Supported SCMs. Currently, `what_git` only supports GitHub and Gitlab.
#[derive(Debug, PartialEq)]
pub enum SCMKind {
    Unsupported,
    GitHub,
    Gitlab,
}

pub type Result = result::Result<SCM, Error>;

#[derive(Debug)]
pub enum Error {
    UnknownProvider(String),
}

impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::UnknownProvider(_) => None,
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::UnknownProvider(ref url) => write!(f, "Unknown provider for url {}", url),
        }
    }
}

mod tests {

    #[test]
    fn test_scrub_git_url() {
        assert_eq!(
            super::scrub_git_url_if_needed("file:///Users/wayfair/foxdie.git"),
            "file:///Users/wayfair/foxdie.git"
        );
        assert_eq!(
            super::scrub_git_url_if_needed("https://github.com/wayfair/foxdie"),
            "https://github.com/wayfair/foxdie"
        );
        assert_eq!(
            super::scrub_git_url_if_needed("https://github.com/wayfair/foxdie.git"),
            "https://github.com/wayfair/foxdie.git"
        );
        assert_eq!(
            super::scrub_git_url_if_needed("git@github.com:wayfair/foxdie"),
            "git://github.com/wayfair/foxdie"
        );
        assert_eq!(
            super::scrub_git_url_if_needed("git@github.com:wayfair/foxdie.git"),
            "git://github.com/wayfair/foxdie.git"
        );
    }
}
