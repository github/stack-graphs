/*--- path: ./A/ModA.ts ---*/
/*--- global: PACKAGE_NAME=foo/bar ---*/

export let a = {
    v: 42
};

/*--- path: ./ModB.ts ---*/
/*--- global: PACKAGE_NAME=foo/bar ---*/

import { a } from "./A/ModA";
//       ^ defined: 4

  a.v;
//^ defined: 11, 4
//  ^ defined: 5
