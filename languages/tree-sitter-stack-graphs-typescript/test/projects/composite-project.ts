/* --- path: tsconfig.json --- */
{
    "compilerOptions": {
        "composite": true,
    }
}

/* --- path: src/foo.ts --- */
export const bar = 42;

/* --- path: src/index.ts --- */
import { bar } from "./foo";
//       ^ defined: 9
