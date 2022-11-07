let x: {
  f: {
    g: number;
  }
};

let y: typeof x.f;
//            ^ defined: 1
//              ^ defined: 2

  y.g;
//^ defined: 7
//  ^ defined: 3

export {};
