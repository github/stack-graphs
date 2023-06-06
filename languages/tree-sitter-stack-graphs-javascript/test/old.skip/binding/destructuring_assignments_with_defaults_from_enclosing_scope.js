let x = { foo: 0 };
let [y, z = x] = [1];

 z.foo;
// ^ defined: 1
