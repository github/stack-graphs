let D = {
    Id: function(x, ignored) {
        return x;
    }
};

  @D.Id
// ^ defined: 1
//   ^ defined: 2
class A {}

  @D.Id()
// ^ defined: 1
//   ^ defined: 2
class B {}

let y = 42;

  @D.Id(y)
// ^ defined: 1
//   ^ defined: 2
class C {}
