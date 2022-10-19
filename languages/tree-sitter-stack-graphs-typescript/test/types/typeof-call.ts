function x(): { g: number } {
  return { g: 42 };
}

let y: typeof x();
//            ^ defined: 1

  y.g;
//^ defined: 5
//  ^ defined: 1

export {};
