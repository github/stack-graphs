type V = { value: number; }

function id<X>(x: X): X; // tsc: error TS2391: Function implementation is missing or not immediately following the declaration.

  id<V>(null).value;
//^ defined: 3
//            ^ defined: 1

export {};
