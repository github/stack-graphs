/*--- path: a.js ---*/

module.exports = function foo() { };

/*--- path: index.js ---*/

module.exports = require("./a.js");

/*--- path: index2.js ---*/

let bar = require("./index.js");

/**/ bar;
//   ^ defined: 3, 7, 11