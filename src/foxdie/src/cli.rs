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
use clap::{crate_version, App, AppSettings, Arg, ArgMatches, SubCommand};

pub fn build_cli<'a, 'b>() -> App<'a, 'b> {
    let args = [
        Arg::with_name("delete")
            .short("D")
            .long("delete")
            .help("Deletes or closes the slate objects under operation. By default, Foxdie will not delete anything without this flag set."),
        Arg::with_name("since")
            .short("s")
            .long("since")
            .required(true)
            .help("Date in RFC 3339 format")
            .takes_value(true)
            .validator(validate_date),
        Arg::with_name("token")
            .short("t")
            .long("token")
            .required(true)
            .help("Personal access token for use with GitHub or Gitlab.")
            .env("TOKEN")
            .hide_env_values(true),
    ];
    App::new("foxdie")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("branches")
                .about("Destroy remote branches from a given Git repository.")
                .long_about("Destroy remote branches from a given Git repository that have not been updated since the specified date.")
                .args(&args)
                .arg(
                    Arg::with_name("DIRECTORY")
                        .help("Sets the Git directory to work from.")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name("push-requests")
                .about("Close push branches filed with a given Git repository URL.")
                .long_about("Close push branches filed with a given Git repository URL that have not been updated since the specified date.")
                .args(&args)
                .arg(
                    Arg::with_name("URL")
                        .help("Sets the URL to a Git repository to work from.")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name("report")
                .about("Generate a JSON report of stale branches from a given Git repository.")
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .help("Output path for the report.")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("DIRECTORY")
                        .help("Sets the Git directory to work from.")
                        .required(true)
                        .index(1),
                ),
        )
        .version(crate_version!())
}

#[allow(clippy::needless_pass_by_value)]
fn validate_date(s: String) -> Result<(), String> {
    DateTime::parse_from_rfc3339(&s)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub struct SharedArguments<'a> {
    pub should_delete: bool,
    pub since: DateTime<FixedOffset>,
    pub token: &'a str,
}

pub fn parse_shared_arguments<'a, 'b>(app_m: &'b ArgMatches<'a>) -> SharedArguments<'b> {
    let should_delete = app_m.is_present("delete");

    let since = app_m
        .value_of("since")
        .and_then(|date_str| DateTime::parse_from_rfc3339(date_str).ok())
        .expect("Should have already validated a date, which is a required argument.");

    let token: &'b str = app_m.value_of("token").expect(
        "Should have passed a token, which is a required argument or environment variable.",
    );

    SharedArguments {
        should_delete,
        since,
        token,
    }
}
