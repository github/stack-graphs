/*--- path: ModA.ts ---*/

export = A;

let A = {
    f: 42
};

/*--- path: ModB.ts ---*/

import a = require('./ModA');

  a.f;
//^ defined: 11, 3
//  ^ defined: 6
