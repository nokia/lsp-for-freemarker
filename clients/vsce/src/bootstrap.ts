/**
 * Copyright 2025-2026 Nokia
 * Licensed under the BSD 3-Clause License.
 * SPDX-License-Identifier: BSD-3-Clause
 */

import { ExtensionContext, Uri, window, workspace } from "vscode";
import * as os from "os";

export async function bootstrap(
    context: ExtensionContext
): Promise<string | undefined> {
    const path = await get_server(context);
    // TODO: check validity
    return path;
}

async function get_server(
    context: ExtensionContext
): Promise<string | undefined> {
    // check if the server path is configured explicitly
    const explicitPath = process.env["__LSP_SERVER_BIN"];
    if (explicitPath) {
        if (explicitPath.startsWith("~/")) {
            return os.homedir() + explicitPath.slice("~".length);
        }
        return explicitPath;
    }

    const ext = process.platform === "win32" ? ".exe" : "";
    const bundled = Uri.joinPath(context.extensionUri, "resources", "bin", `lsp-for-freemarker${ext}`);
    const bundledExists = await fileExists(bundled);

    if (!bundledExists) {
        await window.showErrorMessage(
            "Unfortunately we don't ship binaries for your platform yet. " +
            "Please build and run the server manually from the source code. " +
            "Or, please create an issue on repository."
        );
        return;
    }

    return bundled.fsPath;
}

async function fileExists(uri: Uri) {
    return await workspace.fs.stat(uri).then(
        () => true,
        () => false,
    );
}

// TODO: check availability of the server binary
