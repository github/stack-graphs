class Foo {
  bar;
  baz = 5;
}

let x = new Foo();
  x.bar;
//  ^ defined: 2
  x.baz;
//  ^ defined: 3
