import { mkdirSync } from 'fs';
import { ExtensionContext, Uri } from 'vscode';

import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
    let path = context.asAbsolutePath("out/bin/tree-sitter-stack-graphs-typescript");
    mkdirSync(context.storageUri.fsPath, { recursive: true });
    let db = Uri.joinPath(context.storageUri, "tree-sitter-stack-graphs-typescript.sqlite").fsPath;
    const serverOptions: ServerOptions = {
        command: path,
        args: ["lsp", "-D", db]
    };

    const clientOptions: LanguageClientOptions = {
    };

    client = new LanguageClient(
        "tree-sitter-stack-graphs-typescript",
        "Stack graphs based navigation for TypeScript",
        serverOptions,
        clientOptions
    );

    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    }
    return client ? client.stop() : undefined;
}
