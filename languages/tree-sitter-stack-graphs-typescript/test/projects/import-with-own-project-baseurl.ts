/* --- path: tsconfig.json --- */
/* --- global: PROJECT_NAME=a --- */
{
    "compilerOptions": {
        "composite": true,
        "baseUrl": "./src/",
    }
}

/* --- path: src/foo.ts --- */
/* --- global: PROJECT_NAME=a --- */
export const bar = 42;

/* --- path: src/index.ts --- */
/* --- global: PROJECT_NAME=a --- */
import { bar } from "foo";
//       ^ defined: 12
