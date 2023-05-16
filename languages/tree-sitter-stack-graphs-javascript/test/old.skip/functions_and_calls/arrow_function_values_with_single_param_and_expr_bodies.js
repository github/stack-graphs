let a = { bar: 1 };
let b = { quux: 2 };

let foo = x =>
  (b.quux, x);
// ^ defined: 2
//   ^ defined: 2
//         ^ defined: 4

foo(a).bar;
//     ^ defined: 1
