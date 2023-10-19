/*--- path: index.js ---*/

export function foo() { }

/*--- path: index2.js ---*/

let mod = await import("./index.js");

mod.foo;
//  ^ defined: 3
