/*--- path: ModA.ts ---*/

export default class T {
    v = 42;
};

/*--- path: ModB.ts ---*/

import T from "./ModA";

let x = T;
//      ^ defined: 9, 3

declare let a: T;
//             ^ defined: 9, 3

  a.v;
//^ defined: 14
//  ^ defined: 4
