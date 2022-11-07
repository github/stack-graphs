interface V { v: number; }
interface N { n: number; }

type T = [V, N];

declare let t: T;

  t[0].v;
//^ defined: 6
//     ^ defined: 1

  t[1].n;
//^ defined: 6
//     ^ defined: 2

export {};
