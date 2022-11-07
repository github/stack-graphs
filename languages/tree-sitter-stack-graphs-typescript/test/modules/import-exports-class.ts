/* --- path: A.ts --- */

export = A;

class A {
    f = 42;
}

/* --- path: B.ts --- */

import C = require("./A");

let c: C = new C();
//     ^ defined: 11, 3, 5
//             ^ defined: 11, 3

  c.f;
//^ defined: 13
//  ^ defined: 6
