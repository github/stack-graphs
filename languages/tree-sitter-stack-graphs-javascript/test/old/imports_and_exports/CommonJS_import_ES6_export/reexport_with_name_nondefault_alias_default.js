/*--- path: a.js ---*/

export function foo() { }

/*--- path: index.js ---*/

export { foo as default } from "./a.js";

/*--- path: index2.js ---*/

let bar = await import("./index.js");

/**/ bar.default;
//   ^ defined: 11
//       ^ defined: 3, 7
