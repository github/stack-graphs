interface V {
  v: number;
}

interface I<T> {
  f: T;
  // ^ defined: 5
}

interface J<U> {
  g: I<U>;
//   ^ defined: 5
//     ^ defined: 10
}

var x: J<V>;
//     ^ defined: 10
//       ^ defined: 1

  x.g.f.v;
//^ defined: 16
//  ^ defined: 11
//    ^ defined: 6
//      ^ defined: 2

export {};
