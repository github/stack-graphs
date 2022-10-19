interface N {
  n: number;
}
interface V {
  v: N;
};

var x: V;
//     ^ defined: 4

var { v } = x;
//          ^ defined: 8

  v.n;
//^ defined: 11
//  ^ defined: 2

var { v: w } = x;
//             ^ defined: 8

  w.n;
//^ defined: 18
//  ^ defined: 2

var { "v": p } = x;
//               ^ defined: 8

  p.n;
//^ defined: 25
//  ^ defined: 2

export {};
