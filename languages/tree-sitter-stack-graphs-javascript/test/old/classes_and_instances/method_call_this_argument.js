let x = {
  foo: { baz: 5 },
  bar: function () {
    return this.foo;
  }
};

x.bar().baz;
//      ^ defined: 2

x["bar"]().baz;
//         ^ defined: 2

let y = [
  { baz: 5 },
  function () {
    return this[0];
  }
];

y[1]().baz;
//     ^ defined: 15
