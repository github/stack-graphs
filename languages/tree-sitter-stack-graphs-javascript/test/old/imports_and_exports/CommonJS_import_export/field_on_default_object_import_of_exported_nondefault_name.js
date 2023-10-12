/*--- path: index.js ---*/

exports.foo = function () { };

/*--- path: index2.js ---*/

let mod = require("./index.js");

mod.foo;
//  ^ defined: 3
