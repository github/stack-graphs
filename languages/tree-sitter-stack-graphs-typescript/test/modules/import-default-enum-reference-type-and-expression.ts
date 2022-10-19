/*--- path: ModA.ts ---*/

enum E {
  C
};

export default E;

/*--- path: ModB.ts ---*/

import E from "./ModA";

let x: E = E.C;
//     ^ defined: 11, 7, 3
//         ^ defined: 11, 7
//           ^ defined: 4
