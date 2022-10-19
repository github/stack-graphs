type V = { value: number; }

let foo: {
    bar: V;
};

  foo["bar"].value;
//^ defined: 3
//           ^ defined: 1

export {};
