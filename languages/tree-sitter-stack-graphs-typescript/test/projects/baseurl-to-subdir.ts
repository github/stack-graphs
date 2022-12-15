/* --- path: ./tsconfig.json --- */
{
    "compilerOptions": {
        "composite": true,
        "baseUrl": "./src/",
    }
}

/* --- path: ./src/foo.ts --- */
export const bar = 42;

/* --- path: ./src/index.ts --- */
import { bar } from "foo";
//       ^ defined: 10
