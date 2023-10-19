/*--- path: index.js ---*/

export default {
    foo: 1
};

/*--- path: index2.js ---*/

import mod from "./index.js";

mod.foo;
//  ^ defined: 4
