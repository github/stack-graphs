/*--- path: ModA.ts ---*/

export let a = {
    v: 42
};

/*--- path: ModB.ts ---*/

import { a as b } from "./ModA";
//       ^ defined: 3

  b.v;
//^ defined: 9, 3
//  ^ defined: 4
