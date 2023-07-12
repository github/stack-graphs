let [x, y = x] = [ // newline here distinguishes the object value from the vars
  { foo: 2 }
];

 y.foo;
// ^ defined: 2
