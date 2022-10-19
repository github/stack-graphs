/*--- path: ModA.ts ---*/

export default T;
//             ^ defined: 6

type T = {
    v: number;
};

/*--- path: ModB.ts ---*/

import T from "./ModA";

declare let a: T;
//             ^ defined: 12, 3, 6

  a.v;
//^ defined: 14
//  ^ defined: 7
