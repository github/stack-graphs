class Foo {
  constructor() {
    this.x = 5;
  }
}

class Bar extends Foo {
  constructor() {

  }
}

let bar = new Bar();
bar.x
//  ^ defined: 3
