/*--- path: ModA.ts ---*/

export default {
    v: 42
};

/*--- path: ModB.ts ---*/

import a from "./ModA";

  a.v;
//^ defined: 9, 3
//  ^ defined: 4
