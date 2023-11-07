/* --- path: acme_foo/tsconfig.json --- */
/* --- global: FILE_PATH=tsconfig.json --- */
/* --- global: PROJECT_NAME=acme_foo --- */

{}

/* --- path: acme_foo/package.json --- */
/* --- global: FILE_PATH=package.json --- */
/* --- global: PROJECT_NAME=acme_foo --- */

{
    "name": "@acme/foo",
    "version": "1.0",
    "main": "./api"
}

/* --- path: acme_foo/api.ts --- */
/* --- global: FILE_PATH=api.ts --- */
/* --- global: PROJECT_NAME=acme_foo --- */

export let x;

/* --- path: acme_foo/core.ts --- */
/* --- global: FILE_PATH=core.ts --- */
/* --- global: PROJECT_NAME=acme_foo --- */

export let x;

/* --- path: bar/tsconfig.json --- */
/* --- global: FILE_PATH=tsconfig.json --- */
/* --- global: PROJECT_NAME=bar --- */

{}

/* --- path: bar/package.json --- */
/* --- global: FILE_PATH=package.json --- */
/* --- global: PROJECT_NAME=bar --- */

{
    "name": "bar",
    "dependencies": {
        "@acme/foo": "1"
    }
}

/* --- path: bar/app.ts --- */
/* --- global: FILE_PATH=app.ts --- */
/* --- global: PROJECT_NAME=bar --- */

import { x } from "@acme/foo/core"
//       ^ defined: 27
