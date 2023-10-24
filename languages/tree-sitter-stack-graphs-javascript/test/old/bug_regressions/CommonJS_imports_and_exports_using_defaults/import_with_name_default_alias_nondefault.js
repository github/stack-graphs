/*--- path: a.js ---*/

module.exports = 1;

/*--- path: b.js ---*/

let foo = require("./a.js");

/**/ foo;
//   ^ defined: 3, 7
