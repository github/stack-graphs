  function Foo() {
    this.bar = 5;
  }

  let x = new Foo();
  x.bar;
//  ^ defined: 2

function Bar(x) {
  this.field = x;
}

let bar = new Bar({ baz: 5 });
bar.field.baz
//        ^ defined: 13
