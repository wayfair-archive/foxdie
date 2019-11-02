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

use crate::services::git;
use reqwest;
use serde_json;
use std::error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum FoxdieError {
    UnsupportedProvider(String),
    Git(git::Error),
    Reqwest(reqwest::Error),
    SerdeJson(serde_json::Error),
    Io(io::Error),
}

impl fmt::Display for FoxdieError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FoxdieError::UnsupportedProvider(ref url) => {
                write!(f, "Unsupported provider for url: {}", url)
            }
            FoxdieError::Git(ref err) => write!(f, "Git error: {}", err),
            FoxdieError::Reqwest(ref err) => write!(f, "Reqwest error: {}", err),
            FoxdieError::SerdeJson(ref err) => write!(f, "Serde JSON error: {}", err),
            FoxdieError::Io(ref err) => write!(f, "Io error: {}", err),
        }
    }
}

impl error::Error for FoxdieError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            FoxdieError::UnsupportedProvider(_) => None,
            FoxdieError::Git(ref err) => Some(err),
            FoxdieError::Reqwest(ref err) => Some(err),
            FoxdieError::SerdeJson(ref err) => Some(err),
            FoxdieError::Io(ref err) => Some(err),
        }
    }
}

impl From<git::Error> for FoxdieError {
    fn from(err: git::Error) -> Self {
        FoxdieError::Git(err)
    }
}

impl From<reqwest::Error> for FoxdieError {
    fn from(err: reqwest::Error) -> Self {
        FoxdieError::Reqwest(err)
    }
}

impl From<serde_json::Error> for FoxdieError {
    fn from(err: serde_json::Error) -> Self {
        FoxdieError::SerdeJson(err)
    }
}

impl From<io::Error> for FoxdieError {
    fn from(err: io::Error) -> Self {
        FoxdieError::Io(err)
    }
}
