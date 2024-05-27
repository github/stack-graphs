/* --- path: src/foo.ts --- */
export const bar = 42;

/* --- path: src/index.ts --- */
import { bar } from "./foo.js";
//       ^ defined: 2
