/*--- path: index.js ---*/

module.exports = {
    foo: 1
};

/*--- path: index2.js ---*/

import mod from "./index.js";

mod.foo;
//  ^ defined: 4
