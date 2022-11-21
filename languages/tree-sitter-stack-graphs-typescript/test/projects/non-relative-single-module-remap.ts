/* --- path: ./tsconfig.json --- */
{
    "compilerOptions": {
        "composite": true,
        "baseUrl": "./",
        "paths": {
            "foo": ["lib/the_foo"]
        },
    }
}

/* --- path: ./lib/the_foo.ts --- */
export const bar = 42;

/* --- path: ./src/index.ts --- */
import { bar } from "foo";
//       ^ defined: 13
