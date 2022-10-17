#!/usr/bin/env node

const spawn = require("child_process").spawn;
const path = require("path");

const tssg = process.platform === "win32"
    ? "tree-sitter-stack-graphs.exe"
    : "tree-sitter-stack-graphs";

spawn(
    path.join(__dirname, "bin", tssg), process.argv.slice(2),
    {
        "stdio": "inherit"
    },
).on('close', process.exit);
