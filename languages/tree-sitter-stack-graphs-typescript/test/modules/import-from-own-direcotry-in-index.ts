/*--- path: foo/index.ts ---*/

import { FOO } from "./bar";
//       ^ defined: 8

/*--- path: foo/bar.ts ---*/

export const FOO = 42;
