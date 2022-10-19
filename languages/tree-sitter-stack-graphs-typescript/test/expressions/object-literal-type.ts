interface V {
    v: number;
}

let p: V;
//     ^ defined: 1

let x = {
    p,
//  ^ defined: 5
    q: p,
//     ^ defined: 5
    "r": p,
//       ^ defined: 5
};

  x.p.v;
//^ defined: 8
//  ^ defined: 9
//    ^ defined: 2

  x.q.v;
//^ defined: 8
//  ^ defined: 11
//    ^ defined: 2

  x.r.v;
//^ defined: 8
//  ^ defined: 13
//    ^ defined: 2

export {};
