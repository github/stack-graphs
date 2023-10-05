/* --- path: foo/package.json --- */
/* --- global: FILE_PATH=package.json --- */
/* --- global: PROJECT_NAME=foo --- */

{
    "name": "foo",
    "version": "1.0"
}

/* --- path: foo/src/index.js --- */
/* --- global: FILE_PATH=src/index.js --- */
/* --- global: PROJECT_NAME=foo --- */

export let x;

/* --- path: acme_foo/package.json --- */
/* --- global: FILE_PATH=package.json --- */
/* --- global: PROJECT_NAME=acme_foo --- */

{
    "name": "@acme/foo",
    "version": "1.0",
    "main": "./api"
}

/* --- path: acme_foo/core.js --- */
/* --- global: FILE_PATH=core.js --- */
/* --- global: PROJECT_NAME=acme_foo --- */

export let x;

/* --- path: acme_foo/api.js --- */
/* --- global: FILE_PATH=api.js --- */
/* --- global: PROJECT_NAME=acme_foo --- */

export let x;

/* --- path: bar/package.json --- */
/* --- global: FILE_PATH=package.json --- */
/* --- global: PROJECT_NAME=bar --- */

{
    "name": "bar",
    "dependencies": {
        "foo": "1",
        "@acme/foo": "1"
    }
}

/* --- path: bar/app.js --- */
/* --- global: FILE_PATH=app.js --- */
/* --- global: PROJECT_NAME=bar --- */

import { x } from "@acme/foo"
//       ^ defined: 36

import { x } from "foo"
//       ^ defined: 14

import { x } from "@acme/foo/core"
//       ^ defined: 30
