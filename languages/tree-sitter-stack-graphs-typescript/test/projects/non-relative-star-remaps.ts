/* --- path: ./tsconfig.json --- */
{
    "compilerOptions": {
        "composite": true,
        "baseUrl": "./",
        "paths": {
            "*": ["lib/*", "ext/*"]
        },
    }
}

/* --- path: ./ext/foo.ts --- */
export const bar = 42;

/* --- path: ./src/index.ts --- */
import { bar } from "foo";
//       ^ defined: 13
