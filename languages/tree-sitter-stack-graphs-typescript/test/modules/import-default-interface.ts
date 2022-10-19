/*--- path: ModA.ts ---*/

export default interface T {
  v: number;
};

/*--- path: ModB.ts ---*/

import T from "./ModA"

declare let x: T;
//             ^ defined: 9, 3

  x.v;
//^ defined: 11
//  ^ defined: 4
