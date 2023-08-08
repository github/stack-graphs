/*--- path: a.js ---*/

exports.foo = 1;
module.exports = {
  bar: 2
};

/*--- path: b.js ---*/

const mod1 = require("a.js");

mod1.foo;
//   ^ defined: 3

mod1.bar;
//   ^ defined: 5
