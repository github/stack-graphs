interface N {
  n: number;
}
interface V {
  v: N;
//   ^ defined: 1
}

let xs: V[];
//      ^ defined: 4

for(let { v } of xs) {
//               ^ defined: 9
  v.n;
//^ defined: 12
//  ^ defined: 2
}

export {};
