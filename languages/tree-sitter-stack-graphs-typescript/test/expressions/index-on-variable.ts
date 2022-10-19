interface V {
  v: number;
}

declare let ys: V[];
//              ^ defined: 1

declare let i: number;

const y = ys[i];
//        ^ defined: 5
//           ^ defined: 8

  y.v;
//^ defined: 10
//  ^ defined: 2

export {};
