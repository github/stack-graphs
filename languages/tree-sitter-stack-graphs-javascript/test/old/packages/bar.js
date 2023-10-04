/* --- path: package.json --- */

{
    "name_missing": "/* name omitted to ensure that doesn't error */",
    "dependencies": {
        "foo": "1",
        "@acme/foo": "1"
    }
}

/* --- path: app.js --- */

import { x } from "@acme/foo"
//       ^ defined: 13

import { x } from "foo"
//       ^ defined: 16

import { x } from "@acme/foo/core"
//       ^ defined: 19
