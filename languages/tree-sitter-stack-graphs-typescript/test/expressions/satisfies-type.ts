type Foo = { foo: number };

let x = { foo: 42 } satisfies Foo;

 x.foo
// ^ defined: 3, 1

export {}
