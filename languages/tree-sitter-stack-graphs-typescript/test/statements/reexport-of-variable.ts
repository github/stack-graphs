/*--- path: ModA.ts ---*/

export let a = {
    f: 42
};

/*--- path: ModB.ts ---*/

export { a } from "./ModA";

/*--- path: ModC.ts ---*/

import { a } from "./ModB";

  a.f;
//^ defined: 13, 9, 3
//  ^ defined: 4
