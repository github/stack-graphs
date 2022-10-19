/*--- path: ./ModA.ts ---*/

let a = {
    v: 42
};

export { a };
//       ^ defined: 3

/*--- path: ./ModB.ts ---*/

import { a } from "./ModA";
//       ^ defined: 7, 3

  a.v;
//^ defined: 12, 7, 3
//  ^ defined: 4
