interface V {
  v: number;
}
class C {
  c: number;
}

interface I<T, U = T> {
  f: T;
  // ^ defined: 8
  g: U;
  // ^ defined: 8
}

var x: I<V>;
//     ^ defined: 8
//       ^ defined: 1

  x.f.v;
//^ defined: 15
//  ^ defined: 9
//    ^ defined: 2
  x.g.v;
//^ defined: 15
//  ^ defined: 11
//    ^ defined: 2

var y: I<V, C>;
//     ^ defined: 8
//       ^ defined: 1
//          ^ defined: 4

  y.f.v;
//^ defined: 28
//  ^ defined: 9
//    ^ defined: 2
  y.g.c;
//^ defined: 28
//  ^ defined: 11
//    ^ defined: 5

export {};
