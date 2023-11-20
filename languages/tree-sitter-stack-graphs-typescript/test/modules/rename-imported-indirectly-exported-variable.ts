/*--- path: ModA.ts ---*/

let a = {
    v: 42
};

export { a };
//       ^ defined: 3

/*--- path: ModB.ts ---*/

import { a as b } from "./ModA";
//       ^ defined: 7, 3

  b.v;
//^ defined: 12, 7, 3
//  ^ defined: 4
