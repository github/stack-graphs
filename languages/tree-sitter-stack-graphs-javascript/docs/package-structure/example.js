/* --- path: foo/package.json --- */
/* --- global: FILE_PATH=package.json --- */
/* --- global: PROJECT_NAME=54EA007B --- */

{
    "name": "foo"
}

/* --- path: foo/index.js --- */
/* --- global: FILE_PATH=index.js --- */
/* --- global: PROJECT_NAME=54EA007B --- */

export function foo() {}

/* --- path: bar/package.json --- */
/* --- global: FILE_PATH=package.json --- */
/* --- global: PROJECT_NAME=202D9AA4 --- */

{
    "name": "bar",
    "dependencies": {
        "foo": ""
    }
}

/* --- path: bar/index.js --- */
/* --- global: FILE_PATH=index.js --- */
/* --- global: PROJECT_NAME=202D9AA4 --- */

import { foo } from "foo"
//       ^ defined: 13
