interface V {
  v: number;
}

interface I<T> {
  f: T;
  // ^ defined: 5
}

interface J<U> extends I<U> {
//                     ^ defined: 5
//                       ^ defined: 10
}

var x: J<V>;
//     ^ defined: 10
//       ^ defined: 1

  x.f.v;
//^ defined: 15
//  ^ defined: 6
//    ^ defined: 2

export {};
