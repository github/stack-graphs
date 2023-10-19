/*--- path: a.js ---*/

export function foo() { }

/*--- path: index.js ---*/

export { foo } from "./a.js";

/*--- path: index2.js ---*/

let { foo } = await import("./index.js");

/**/ foo;
//   ^ defined: 3, 7, 11
