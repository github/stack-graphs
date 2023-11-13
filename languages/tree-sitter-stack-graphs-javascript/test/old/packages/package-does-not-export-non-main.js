/* --- path: foo/package.json --- */
/* --- global: FILE_PATH=package.json --- */
/* --- global: PROJECT_NAME=foo --- */

{
    "name": "foo",
    "version": "1.0"
}

/* --- path: foo/impl.js --- */
/* --- global: FILE_PATH=impl.js --- */
/* --- global: PROJECT_NAME=foo --- */

export let x;

/* --- path: bar/package.json --- */
/* --- global: FILE_PATH=package.json --- */
/* --- global: PROJECT_NAME=bar --- */

{
    "name": "bar",
    "dependencies": {
        "foo": "1"
    }
}

/* --- path: bar/app.js --- */
/* --- global: FILE_PATH=app.js --- */
/* --- global: PROJECT_NAME=bar --- */

import { x } from "foo"
//       ^ defined:
