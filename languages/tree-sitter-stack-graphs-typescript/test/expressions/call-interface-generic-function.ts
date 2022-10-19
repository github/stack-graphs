type V = { value: number; }

interface I {
    id<X>(x: X): X;
}

let x:I;

  x.id<V>(null).value;
//^ defined: 7
//  ^ defined: 4
//              ^ defined: 1

export {};
