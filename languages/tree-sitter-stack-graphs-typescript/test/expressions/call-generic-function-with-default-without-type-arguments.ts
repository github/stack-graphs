interface V {
  v: number;
}

let id: <X = V>(x: X) => X;
//                 ^ defined: 5
//                       ^ defined: 5

  id(null).v;
//^ defined: 5
//         ^ defined: 2

export {};
