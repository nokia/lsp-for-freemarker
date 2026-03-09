// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use tokio::sync::{OnceCell, SetError};
use tower_lsp_server::{self, Client};

static CLIENT_ONCE: OnceCell<Client> = OnceCell::const_new();

pub fn save_client(c: Client) -> Result<(), SetError<Client>> {
    CLIENT_ONCE.set(c)
}

pub fn get_client() -> Option<&'static Client> {
    CLIENT_ONCE.get()
}

#[macro_export]
macro_rules! window_log_info {
    ($message:expr) => {
        if let Some(c) = $crate::client::get_client() {
            c.log_message(tower_lsp_server::ls_types::MessageType::INFO, $message)
                .await;
        }
    };
}

#[macro_export]
macro_rules! window_log_warn {
    ($message:expr) => {
        if let Some(c) = $crate::client::get_client() {
            c.log_message(tower_lsp_server::ls_types::MessageType::WARNING, $message)
                .await;
        }
    };
}

#[macro_export]
macro_rules! window_log_error {
    ($message:expr) => {
        if let Some(c) = $crate::client::get_client() {
            c.log_message(tower_lsp_server::ls_types::MessageType::WARNING, $message)
                .await;
        }
    };
}
