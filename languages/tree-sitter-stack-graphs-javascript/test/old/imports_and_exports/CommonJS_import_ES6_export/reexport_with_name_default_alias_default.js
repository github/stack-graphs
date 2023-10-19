/*--- path: a.js ---*/

export default function foo() { }

/*--- path: index.js ---*/

export { default as default } from "./a.js";

/*--- path: index2.js ---*/

let bar = await import("./index.js");

/**/ bar.default;
//   ^ defined: 11
//       ^ defined: 3, 7
