type V = { value: number; }

function id<X>(x: X): X {
    return x;
}

  id<V>(null).value;
//^ defined: 3
//            ^ defined: 1

export {};
