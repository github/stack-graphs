function foo({ x: xval }) {
  return xval;
//       ^ defined: 1
}

foo({ x: { y: 1 } }).y;
//                   ^ defined: 6
