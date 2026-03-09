/**
 * Copyright 2025-2026 Nokia
 * Licensed under the BSD 3-Clause License.
 * SPDX-License-Identifier: BSD-3-Clause
 */

import {
    commands,
    ExtensionContext,
    LogLevel,
    RelativePattern,
    TextDocument,
    Uri,
    window,
    workspace,
    WorkspaceFolder,
    WorkspaceFoldersChangeEvent,
} from "vscode";

import { Executable, LanguageClient, LanguageClientOptions, ServerOptions, Trace } from "vscode-languageclient/node";

import { bootstrap } from "./bootstrap";

const extensionId = "lsp-for-freemarker";
const language_server_name = "Freemarker Language Server";
const outputChannel = window.createOutputChannel(extensionId, { log: true });

const clients: Map<string, LanguageClient> = new Map();
let binary_path: String | undefined = undefined;

function window_log_message(msg: String) {
    outputChannel.appendLine(`[${extensionId}]: ${msg}`);
}

function freemarker_file_pattern(folder: Uri) {
    return new RelativePattern(folder, "**/*.ftl");
}

async function open_document(uri: Uri) {
    const uriMatch = (d: TextDocument) => d.uri.toString() === uri.toString();
    const doc = workspace.textDocuments.find(uriMatch);
    if (doc === undefined) {
        await workspace.openTextDocument(uri);
    }
    return uri;
}

async function start_client_for_folder(
    folder: WorkspaceFolder,
    ctx: ExtensionContext
) {
    window_log_message(`Prepare client for folder: ${folder.name}`);
    const root = folder.uri;

    const deleteWatcher = workspace.createFileSystemWatcher(
        freemarker_file_pattern(root),
        true, // ignoreCreateEvents
        true, // ignoreChangeEvents
        false // ignoreDeleteEvents
    );

    const createChangeWatcher = workspace.createFileSystemWatcher(
        freemarker_file_pattern(root),
        false, // ignoreCreateEvents
        false, // ignoreChangeEvents
        true // ignoreDeleteEvents
    );

    ctx.subscriptions.push(deleteWatcher);
    ctx.subscriptions.push(createChangeWatcher);
    // TODO: figure out if below 2 subscription are indeed useful
    //ctx.subscriptions.push(createChangeWatcher.onDidCreate(open_document));
    //ctx.subscriptions.push(createChangeWatcher.onDidChange(open_document));

    const server_exec: Executable = {
        command: String(binary_path),
        options: {
            env: {
                ...process.env
            },
        }
    };

    const server_opt: ServerOptions = {
        run: server_exec,
        debug: server_exec,
    };

    let client_opt: LanguageClientOptions = {
        documentSelector: [
            { language: "ftl", pattern: `${root.fsPath}/**/*.ftl`, scheme: "file" },
        ],
        synchronize: {
            fileEvents: [
                createChangeWatcher,
                deleteWatcher
            ]
        },
        initializationFailedHandler: (err) => {
            window_log_message(`Failed to initialize language client: ${err}.`);
            return true;
        },
        diagnosticCollectionName: extensionId,
        workspaceFolder: folder,
        outputChannel,
    };

    const client = new LanguageClient(
        extensionId,
        extensionId,
        server_opt,
        client_opt,
    );

    window_log_message(`Starting client...`);
    await client.start();
    window_log_message(`Client started.`);
}

function stop_client(client: LanguageClient) {
    client.diagnostics?.clear();
    return client.stop();
}

async function stop_client_for_folder(workspaceFolder: string) {
    window_log_message(`Stop client for folder: ${workspaceFolder}`);
    const client = clients.get(workspaceFolder);
    if (client) {
        await stop_client(client);
    }
    clients.delete(workspaceFolder);
}

function update_clients(context: ExtensionContext) {
    return async function ({ added, removed }: WorkspaceFoldersChangeEvent) {
        for (const folder of removed) {
            await stop_client_for_folder(folder.uri.toString());
        }
        for (const folder of added) {
            await start_client_for_folder(folder, context);
        }
    };
}

export async function activate(context: ExtensionContext): Promise<void> {
    window_log_message(`Extension activating...`);
    binary_path = await bootstrap(context);
    if (!binary_path) {
        await window.showErrorMessage(
            `Not starting ${language_server_name} as a suitable binary was not found.`
        );
        return;
    }
    window_log_message(`Found ${language_server_name} binary: ${binary_path}`);

    context.subscriptions.push(
        workspace.onDidChangeWorkspaceFolders(update_clients(context))
    );

    const folders = workspace.workspaceFolders || [];
    for (const folder of folders) {
        await start_client_for_folder(folder, context);
    }

    commands.registerCommand("freemarker.restart", async () => {
        const currentFolder = workspace.workspaceFolders?.[0].uri.toString();
        if (!currentFolder) {
            await window.showInformationMessage("No Freemarker workspace is open.");
            return;
        }
        const client = clients.get(currentFolder);
        if (client) {
            const folder = client.clientOptions.workspaceFolder;
            await stop_client(client);
            if (folder) {
                await start_client_for_folder(folder, context);
            }
        }
    });
    window_log_message(`Extension activatied.`);
}

export async function deactivate(): Promise<void> {
    await Promise.all([...clients.values()].map(stop_client));
}
