import { workspace, ExtensionContext } from 'vscode';

import {
    Executable,
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
    let path = context.asAbsolutePath("out/bin/tree-sitter-stack-graphs-typescript");
    const serverOptions: ServerOptions = {
        command: path,
        args: ["lsp"]
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
    if (!client) {
        return undefined;
    }
    return client.stop();
}
