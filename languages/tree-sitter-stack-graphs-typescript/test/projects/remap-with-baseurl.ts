/* --- path: ./tsconfig.json --- */
{
    "compilerOptions": {
        "composite": true,
        "baseUrl": "./src",
        "paths": {
            "util/*": ["util_impl/*"]
        },
    }
}

/* --- path: ./src/util_impl/foo.ts --- */
export const bar = 42;

/* --- path: ./src/index.ts --- */
import { bar } from "util/foo";
//       ^ defined: 13
