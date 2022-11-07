class V {
  value: number = 1;
}

let VV = V;
//       ^ defined: 1

new VV().value;
//  ^ defined: 5
//       ^ defined: 2

export {};
