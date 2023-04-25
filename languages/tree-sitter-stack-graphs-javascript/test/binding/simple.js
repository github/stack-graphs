var x = 1;
let y = 2;
const z = 3;

/**/ x + y + z;
//   ^ defined: 1
//       ^ defined: 2
//           ^ defined: 3

x = (y = 2);

/**/ x + y;
//   ^ defined: 1, 10
//       ^ defined: 2, 10

/**/ x *= (z += 2);
//   ^ defined: 1, 10
//         ^ defined: 3

/**/ z - x;
//   ^ defined: 3, 16
//       ^ defined: 1, 10, 16

let a = 1;
let b = a;

/**/ b;
//   ^ defined: 24, 25
// this seems weird but it makes sense in terms of what kinds of results we want
// to find: both the assignment to b, and also what it ultimately resolves to
