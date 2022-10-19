interface V { v: number; }
interface N { n: number; }

type T = [V, N];

declare let t: T;

let [x, y] = t;

  x.v;
//^ defined: 8
//  ^ defined: 1

  y.n;
//^ defined: 8
//  ^ defined: 2

export {};
