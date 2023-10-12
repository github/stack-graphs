/*--- path: index.js ---*/

export default function foo() { }

/*--- path: index2.js ---*/

let bar = await import("./index.js");

/**/ bar.default;
//   ^ defined: 7
//       ^ defined: 3
