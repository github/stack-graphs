/*--- path: a.js ---*/

module.exports.foo = 1;

/*--- path: b.js ---*/

let mod = require("./a.js");

mod.foo;
//  ^ defined: 3