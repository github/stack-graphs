import { homedir } from 'os';
import { ExtensionContext, StatusBarItem, Uri, commands, window, workspace } from 'vscode';

// should match `name` in `Cargo.toml`
const NAME = "tree-sitter-stack-graphs-typescript";

import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    integer
} from 'vscode-languageclient/node';

let client: LanguageClient;
let status: StatusBarItem;

export function activate(context: ExtensionContext) {
    let storageUri = context?.storageUri;
    if (!storageUri) {
        // cannot start, no workspace is open
        return;
    }
    // ensure workspace folder exists
    workspace.fs.createDirectory(storageUri);

    let command = context.asAbsolutePath("out/bin/" + NAME);
    let args = buildArgsFromConfig(storageUri);
    const serverOptions: ServerOptions = { command, args };
    const clientOptions: LanguageClientOptions = {
        // these should match `file_types` and `special_files` in `rust/lib.rs`
        documentSelector: [
            { scheme: 'file', pattern: "**/*.ts" },
            { scheme: 'file', pattern: "**/tsconfig.json" },
            { scheme: 'file', pattern: "**/package.json" }
        ]
    };

    client = new LanguageClient(
        NAME,
        NAME,
        serverOptions,
        clientOptions
    );

    status = window.createStatusBarItem();
    status.text = NAME;
    status.show();

    client.start();

    workspace.onDidChangeConfiguration((e) => {
        if (e.affectsConfiguration(NAME)) {
            serverOptions.args = buildArgsFromConfig(storageUri);
            client.restart();
        }
    });
}

function buildArgsFromConfig(storageUri: Uri): string[] {
    let args = ["lsp"];

    let config = workspace.getConfiguration(NAME);
    let config_db_path = config.get<string>('database.path');
    if (config_db_path) {
        let db_path = config_db_path.replace(/^~(?=$|\/|\\)/, homedir());
        args.push("-D", db_path);
    } else {
        switch (config.get<string>('database.defaultLocation')) {
            case "workspace":
                let db_path = Uri.joinPath(storageUri, NAME + ".sqlite").fsPath;
                args.push("-D", db_path);
                break;
            case "user":
                // omit -D
                break;
        }
    }

    let max_folder_index_time = config.get<integer>('index.maxFolderTime');
    if (max_folder_index_time && max_folder_index_time >= 0) {
        args.push("--max-folder-index-time", max_folder_index_time.toString());
    }
    let max_file_index_time = config.get<integer>('index.maxFileTime');
    if (max_file_index_time && max_file_index_time >= 0) {
        args.push("--max-file-index-time", max_file_index_time.toString());
    }
    let max_query_time = config.get<integer>('query.maxTime');
    if (max_query_time && max_query_time >= 0) {
        args.push("--max-query-time", max_query_time.toString());
    }

    return args;
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    if (status) {
        status.dispose();
    }
    let result = client.stop();
    client = undefined;
    return result;
}
