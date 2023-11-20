/* --- path: foo.ts --- */

export const FOO = 42;

/* --- path: bar.ts --- */

export * as quz from "./foo";

/* --- path: test.ts --- */

import { quz } from "./bar";

   quz.FOO
// ^ defined: 11, 7
//     ^ defined: 3
