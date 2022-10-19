let x: { f: string };

let y: typeof x = { f: "fortytwo" };
//            ^ defined: 1

  y.f;
//^ defined: 3
//  ^ defined: 1

export {};
