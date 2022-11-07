/*--- path: ModA.ts ---*/

export let x = {
  v: 42
};

/*--- path: ModB.ts ---*/

import * as A from "./ModA";

  A.x.v;
//^ defined: 9
//  ^ defined: 3
//    ^ defined: 4
