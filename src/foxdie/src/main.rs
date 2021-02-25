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

mod actions;
mod cli;
mod error;
mod services;

use cli::{build_cli, parse_shared_arguments, SharedArguments};
use log::{error, warn};
use std::env;
use std::process;

#[tokio::main]
async fn main() {
    setup_logger();
    let app = build_cli();
    let app_m = app.get_matches();
    let res = run_matches(&app_m).await;
    if let Err(err) = res {
        error!("{}", err);
        process::exit(1);
    }
}

async fn run_matches(args: &clap::ArgMatches<'_>) -> Result<(), error::FoxdieError> {
    match args.subcommand() {
        ("branches", Some(sub_m)) => {
            let SharedArguments {
                should_delete,
                since,
                token,
            } = parse_shared_arguments(&sub_m);
            let path = sub_m.value_of("DIRECTORY");
            if !should_delete {
                print_dry_run_warning();
            }
            actions::local::clean_remote_branches(
                path,
                actions::local::Options {
                    should_delete,
                    since_date: &since,
                    token,
                },
            )
            .await
        }
        ("push-requests", Some(sub_m)) => {
            let SharedArguments {
                should_delete,
                since,
                token,
            } = parse_shared_arguments(&sub_m);
            if !should_delete {
                print_dry_run_warning();
            }
            let url = sub_m
                .value_of("URL")
                .expect("URL was supposed to be passed as a positional argument.");
            actions::push_requests::clean_push_requests(should_delete, &since, &url, &token).await
        }
        ("report", Some(sub_m)) => {
            let output_path = sub_m.value_of("output");
            let repo_path = sub_m.value_of("DIRECTORY");
            actions::report::report(&output_path, repo_path)
        }
        _ => unreachable!(),
    }
}

fn setup_logger() {
    let rust_log = match env::var("RUST_LOG") {
        Ok(var) => var,
        _ => String::from("foxdie=info"),
    };
    env_logger::builder()
        .format_module_path(false)
        .format_timestamp(None)
        .parse_filters(&rust_log)
        .init();
}

fn print_dry_run_warning() {
    warn!(
        "Foxdie is being run in dry run mode, which is the default. \
         If this is undesirable, run again with the `--delete` flag."
    );
}
