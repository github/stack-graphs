/*--- path: a.js ---*/

export default function foo() { }

/*--- path: index.js ---*/

export { default as bar } from "./a.js";

/*--- path: index2.js ---*/

let { bar } = await import("./index.js");

/**/ bar;
//   ^ defined: 3, 7, 11
