interface V {
  v: number;
}

let xs: V[];
//      ^ defined: 1

for(let x of xs) {
//           ^ defined: 5
  x.v;
//^ defined: 8
//  ^ defined: 2
}

export {};
