/* --- path: ./tsconfig.json --- */
{
    "compilerOptions": {
        "composite": true,
        "baseUrl": "./",
        "paths": {
            "*": ["lib/*"]
        },
    }
}

/* --- path: ./lib/foo.ts --- */
export const bar = 42;

/* --- path: ./src/index.ts --- */
import { bar } from "foo";
//       ^ defined: 13
