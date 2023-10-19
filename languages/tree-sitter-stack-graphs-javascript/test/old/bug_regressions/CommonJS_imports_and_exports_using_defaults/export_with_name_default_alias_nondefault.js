/*--- path: a.js ---*/

module.exports = 1;

/*--- path: b.js ---*/

exports.foo = require("./a.js");

/*--- path: c.js ---*/

let { foo } = require("./b.js");

/**/ foo;
//   ^ defined: 3, 7, 11
