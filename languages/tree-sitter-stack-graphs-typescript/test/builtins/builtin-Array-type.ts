interface V {
  v: number;
}

declare let xs: Array<V>;
//                    ^ defined: 1

  xs[0].v;
//^ defined: 5
//      ^ defined: 2

  xs.find((x) => x.v === 42).v;
//^ defined: 5
//               ^ defined: 12
//                 ^ defined: 2
//                           ^ defined: 2

export {};
