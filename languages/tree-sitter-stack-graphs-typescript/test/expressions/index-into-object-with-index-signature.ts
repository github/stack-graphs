type V = { value: number; }

let foo: {
    [index:number]: V;
};

  foo[1].value;
//^ defined: 3
//       ^ defined: 1

export {};
