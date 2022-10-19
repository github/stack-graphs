interface V {
  v: number;
}

let f = x => x;
//           ^ defined: 5

let g = (y:V) => y;
//         ^ defined: 1
//               ^ defined: 8

  g(null).v;
//^ defined: 8
//        ^ defined: 2

let h = (x):V => x;
//          ^ defined: 1
//               ^ defined: 16

export {};
