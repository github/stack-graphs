#!/usr/bin/env node

const child_process = require("child_process");
const packageJSON = require("./package.json");

const cargo = process.platform === "win32"
    ? "cargo.exe"
    : "cargo";

try {
    child_process.execSync(cargo);
} catch (error) {
    console.error(error.message);
    console.error("Failed to execute Cargo. Cargo needs to be available to install this package!");
    process.exit(1);
}

child_process.spawn(
    cargo, [
        "install",
        "--quiet",
        "--root", ".",
        "--version", "^"+packageJSON.version,
        "--features", "cli",
        packageJSON.name,
    ],
    {
        "stdio": "inherit"
    },
).on('close', process.exit);
