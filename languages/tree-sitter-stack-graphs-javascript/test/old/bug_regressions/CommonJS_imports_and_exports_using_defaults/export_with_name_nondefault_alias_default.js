/*--- path: a.js ---*/

exports.foo = 1;

/*--- path: b.js ---*/

let { foo } = require("./a.js");
module.exports = foo;

/*--- path: c.js ---*/

let bar = require("./b.js");

/**/ bar;
//   ^ defined: 3, 7, 8, 12
