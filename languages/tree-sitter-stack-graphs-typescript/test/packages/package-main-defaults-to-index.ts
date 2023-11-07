/* --- path: foo/tsconfig.json --- */
/* --- global: FILE_PATH=tsconfig.json --- */
/* --- global: PROJECT_NAME=foo --- */

{}

/* --- path: foo/package.json --- */
/* --- global: FILE_PATH=package.json --- */
/* --- global: PROJECT_NAME=foo --- */

{
    "name": "foo",
    "version": "1.0"
}

/* --- path: foo/index.ts --- */
/* --- global: FILE_PATH=index.ts --- */
/* --- global: PROJECT_NAME=foo --- */

export let x;

/* --- path: foo/impl.ts --- */
/* --- global: FILE_PATH=impl.ts --- */
/* --- global: PROJECT_NAME=foo --- */

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
        "foo": "1"
    }
}

/* --- path: bar/app.ts --- */
/* --- global: FILE_PATH=app.ts --- */
/* --- global: PROJECT_NAME=bar --- */

import { x } from "foo"
//       ^ defined: 20
