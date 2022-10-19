/*--- path: ./ModA.ts ---*/

export let a = {
    v: 42
};

/*--- path: ./ModB.ts ---*/

import { a } from "./ModA";
//       ^ defined: 3

  a.v;
//^ defined: 9, 3
//  ^ defined: 4
