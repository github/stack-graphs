/*--- path: index.js ---*/

export function foo() { }

/*--- path: index2.js ---*/

let { foo } = await import("./index.js");

/**/ foo;
//   ^ defined: 3, 7
