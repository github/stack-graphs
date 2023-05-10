class Foo {
  bar(x) {
    return x;
  }
}

let obj = { field: 5 };
let y = new Foo();
 y.bar;
// ^ defined: 2
y.bar(obj).field
//         ^ defined: 7
