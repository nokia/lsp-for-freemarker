// Copyright 2025-2026 Nokia
// Licensed under the BSD 3-Clause License.
// SPDX-License-Identifier: BSD-3-Clause

use std::fmt::Display;

use tokio::sync::OnceCell;
use tower_lsp_server::{Client, ls_types::MessageType};

static CLIENT_INSTANCE: OnceCell<Client> = OnceCell::const_new();

pub(crate) fn init(c: Client) {
    let _ = CLIENT_INSTANCE.set(c);
}

/// Sends window/logMessage notifications to the client.
pub(crate) struct Window;

#[macro_export]
macro_rules! info {
    ($msg:expr) => {
        (
            tower_lsp_server::ls_types::MessageType::INFO,
            $msg.to_owned(),
        )
    };
}

#[macro_export]
macro_rules! warning {
    ($msg:expr) => {
        (
            tower_lsp_server::ls_types::MessageType::WARNING,
            $msg.to_owned(),
        )
    };
}

#[macro_export]
macro_rules! error {
    ($msg:expr) => {
        (
            tower_lsp_server::ls_types::MessageType::ERROR,
            $msg.to_owned(),
        )
    };
}

impl Window {
    /// Logs a message to the client window, spawning a task so it can be called
    /// from synchronous contexts.
    #[allow(unused)]
    pub(crate) fn log_sync<M: Display>(m: (MessageType, M)) {
        if let Some(client) = CLIENT_INSTANCE.get() {
            let client = client.clone();
            let message = m.1.to_string();
            tokio::spawn(async move { client.log_message(m.0, message).await });
        }
    }

    /// Logs a message to the client window.
    pub(crate) async fn log<M: Display>(m: (MessageType, M)) {
        if let Some(client) = CLIENT_INSTANCE.get() {
            client.log_message(m.0, m.1).await;
        }
    }
}
