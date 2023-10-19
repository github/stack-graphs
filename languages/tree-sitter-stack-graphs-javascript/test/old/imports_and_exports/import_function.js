/*--- path: a.js ---*/

exports.foo = 2;
module.exports = 1;

/*--- path: b.js ---*/

let mod = await import("./a.js");

mod.foo;
//  ^ defined: 3

mod.default;
//  ^ defined: 3, 4
//  !!!! TODO 3 is here because the `exports.foo` on line 3 also defines
//  the default object. this is a current limitation of the import/export
//  system to support CommonJS behavior

mod.default.foo;
//          ^ defined: 3
