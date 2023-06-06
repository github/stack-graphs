let a = { bar: 1 };
let b = { quux: 2 };

let foo = function bar(x) {
   b.quux;
// ^ defined: 2
//   ^ defined: 2
   bar;
// ^ defined: 4
  return x;
//       ^ defined: 4
};

foo(a).bar;
//     ^ defined: 1
