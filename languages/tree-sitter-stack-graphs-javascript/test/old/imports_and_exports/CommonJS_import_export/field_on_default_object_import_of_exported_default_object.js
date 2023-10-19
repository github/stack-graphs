/*--- path: index.js ---*/

module.exports = {
    foo: 1
};

/*--- path: index2.js ---*/

let mod = require("./index.js");

mod.foo;
//  ^ defined: 4
