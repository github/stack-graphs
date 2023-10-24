/*--- path: a.js ---*/

export default 1;

/*--- path: b.js ---*/

import { default as foo } from "./a.js";

/**/ foo;
//   ^ defined: 3, 7
