/*--- path: ModA.ts ---*/

export default {
    f: 42
};

/*--- path: ModB.ts ---*/

export { default as b } from "./ModA";

/*--- path: ModC.ts ---*/

import { b } from "./ModB";

  b.f;
//^ defined: 13, 9, 3
//  ^ defined: 4
