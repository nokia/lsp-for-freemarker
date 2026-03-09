// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]

use std::env;
use tower_lsp_server::LspService;
use tracing::{level_filters::LevelFilter, subscriber};
use tracing_subscriber::fmt::format::FmtSpan;

mod action;
mod analysis;
mod client;
mod completion;
mod diagnosis;
mod doc;
mod folding;
mod format;
mod goto;
mod hover;
mod init;
mod parser;
mod reactor;
mod server;
mod symbol;
mod tokenizer;
mod utils;
mod workspace;

#[tokio::main]
async fn main() {
    // tracing facility
    let cache_dir = env::temp_dir().join(server::Server::CODE_NAME);
    let file_appender = tracing_appender::rolling::hourly(cache_dir, "lsp-for-freemarker.log");
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(file_appender);

    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::fmt()
        // Use a more compact, abbreviated log format
        .compact()
        // Display source code file paths
        .with_file(true)
        // Display source code line numbers
        .with_line_number(true)
        // Don't display the thread ID an event was recorded on
        .with_thread_ids(false)
        // Don't display the event's target (module path)
        .with_target(false)
        // Log when entering and exiting spans
        .with_span_events(FmtSpan::ACTIVE)
        // log to a file
        .with_writer(non_blocking_writer)
        // Disabled ANSI color codes for better compatibility with some terminals
        .with_ansi(false)
        // TODO: log level control
        .with_max_level(LevelFilter::INFO)
        // Build the subscriber
        .finish();

    // use that subscriber to process traces emitted after this point
    subscriber::set_global_default(subscriber).expect("Could not set global default subscriber");

    // TODO: support other commands (e.g. `--version`, `--log`)
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(server::Server::new);
    tower_lsp_server::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}
