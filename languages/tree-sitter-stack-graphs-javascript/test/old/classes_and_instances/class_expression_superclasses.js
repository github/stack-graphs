let Foo = class {
  constructor() {
    this.x = 5;
  }
}

let Bar = class extends Foo {
  constructor() {

  }
}

let bar = new Bar();
bar.x
//  ^ defined: 3
