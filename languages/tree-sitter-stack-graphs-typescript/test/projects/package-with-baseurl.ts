/* --- path: ./package.json --- */
{
    "name": "@my/pkg"
}

/* --- path: ./tsconfig.json --- */
{
}

/* --- path: ./src/foo.ts --- */
export const bar = 42;

/* --- path: ./src/index.ts --- */
import { bar } from "@my/pkg/foo";
//       ^ defined: 11
