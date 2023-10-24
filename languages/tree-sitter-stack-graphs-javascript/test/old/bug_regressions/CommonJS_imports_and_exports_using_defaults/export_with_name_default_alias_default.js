/*--- path: a.js ---*/

module.exports = 1;

/*--- path: b.js ---*/

module.exports = require("./a.js");

/*--- path: c.js ---*/

let bar = require("./b.js");

/**/ bar;
//   ^ defined: 3, 7, 11
