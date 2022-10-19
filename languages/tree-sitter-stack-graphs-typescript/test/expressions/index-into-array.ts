interface V { v: number; }
interface N { n: number; }

let xs = [{ v: 42 }, { n: 42 }];

  xs[0].v;
//^ defined: 4
//      ^ defined: 4

  xs[0].n;
//^ defined: 4
//      ^ defined: 4

declare let i: number;

  xs[i].n;
//^ defined: 4
//      ^ defined: 4

export {};
