/*--- path: ModA.ts ---*/

class T {
    v = 42;
};

export default T;

/*--- path: ModB.ts ---*/

import T from "./ModA";

let x = T;
//      ^ defined: 11, 7

declare let a: T;
//             ^ defined: 11, 7, 3

  a.v;
//^ defined: 16
//  ^ defined: 4
