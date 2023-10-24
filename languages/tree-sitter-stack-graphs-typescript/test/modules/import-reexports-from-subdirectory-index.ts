/* --- path: foo/index.ts --- */

export * from './bar'

/* --- path: foo/bar.ts --- */

export let baz = 42

/* --- path: qux.ts --- */

import { baz } from './foo'
//       ^ defined: 7
