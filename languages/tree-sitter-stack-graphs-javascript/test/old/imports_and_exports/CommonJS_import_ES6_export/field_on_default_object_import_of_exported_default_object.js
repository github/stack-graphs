/*--- path: index.js ---*/

export default {
    foo: 1
};

/*--- path: index2.js ---*/

let mod = await import("./index.js");

mod.default.foo;
//          ^ defined: 4
