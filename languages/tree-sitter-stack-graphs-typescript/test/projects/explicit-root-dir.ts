/* --- path: ./tsconfig.json --- */
{
    "compilerOptions": {
        "rootDir": "."
    }
}

/* --- path: ./core/foo.ts --- */
export const bar = 42;

/* --- path: ./core/index.ts --- */
import { bar } from "./foo";
//       ^ defined: 9
