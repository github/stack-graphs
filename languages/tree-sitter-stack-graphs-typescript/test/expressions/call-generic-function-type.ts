type V = { value: number; }

let id: <X>(x: X) => X;

  id<V>(null).value;
//^ defined: 3
//            ^ defined: 1

export {};
