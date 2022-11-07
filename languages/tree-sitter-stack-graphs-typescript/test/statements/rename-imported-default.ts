/*--- path: ModA.ts ---*/

export default {
    f: 42
};

/*--- path: ModB.ts ---*/

import { default as b } from "./ModA";

  b.f;
//^ defined: 9, 3
//  ^ defined: 4
