/* --- path: a/foo.ts --- */
/* --- global: PROJECT_NAME=a */

export const baz = 42;

/* --- path: b/bar.ts --- */
/* --- global: PROJECT_NAME=b */

import { baz } from "./foo";
//       ^ defined: