/*--- path: A/ModA.ts ---*/

export let a = {
    v: 42
};

/*--- path: B/ModB.ts ---*/

import { a } from "../A/ModA";
//       ^ defined: 3

  a.v;
//^ defined: 9, 3
//  ^ defined: 4
