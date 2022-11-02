/*--- path: ./A/ModA.ts ---*/
/*--- global: PACKAGE_NAME=baz ---*/

export let a = {
    v: 42
};

/*--- path: ./ModB.ts ---*/
/*--- global: PACKAGE_NAME=foo/bar ---*/

import { a } from "baz/A/ModA";
//       ^ defined: 4

  a.v;
//^ defined: 11, 4
//  ^ defined: 5
