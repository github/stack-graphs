/*--- path: A/index.ts ---*/

export let a = {
    v: 42
};

/*--- path: B/ModB.ts ---*/

import { a } from "../A";
//       ^ defined: 3

  a.v;
//^ defined: 9, 3
//  ^ defined: 4
