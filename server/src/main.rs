// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]

use tower_lsp_server::LspService;

mod action;
mod client;
mod completion;
mod consts;
mod diagnosis;
mod document;
mod features;
mod folding;
mod format;
mod goto;
mod grammar;
mod hover;
mod href;
mod init;
mod semantic;
mod server;
mod syntax;
mod text;
mod tokenizer;
mod utils;

#[tokio::main]
async fn main() {
    // TODO: support other commands (e.g. `--version`, `--log`)
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(server::Server::new);
    tower_lsp_server::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}
