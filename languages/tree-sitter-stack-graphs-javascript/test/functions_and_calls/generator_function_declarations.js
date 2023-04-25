let a = { bar: 1 };
let b = { quux: 2 };

function* foo(x) {
   b.quux;
// ^ defined: 2
//   ^ defined: 2
  yield x;
//      ^ defined: 4
}

foo(a).bar;
//     ^ defined: 1
