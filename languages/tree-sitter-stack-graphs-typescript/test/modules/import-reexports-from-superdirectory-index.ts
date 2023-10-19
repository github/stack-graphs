/* --- path: index.ts --- */

export * from './bar'

/* --- path: bar.ts --- */

export let baz = 42

/* --- path: foo/qux.ts --- */

import { baz } from '..'
//       ^ defined: 7
