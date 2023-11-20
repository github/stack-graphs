/* --- path: tsconfig.json --- */
{
    "compilerOptions": {
        "rootDirs": [
            "src/core",
            "src/util"
        ]
    }
}

/* --- path: src/core/index.ts --- */
import { bar } from "./foo/baz";
//       ^ defined: 16

/* --- path: src/util/foo/baz.ts --- */
export const bar = 42;

/* --- path: src/util/index.ts --- */

/* --- path: src/index.ts --- */
