function Id(x, ignored) {
    return x;
}

  @Id
// ^ defined: 1
class A {
}

  @Id()
// ^ defined: 1
class B {
}

let y = 42;

  @Id(y)
// ^ defined: 1
//    ^ defined: 15
class C {
}

class D {
      @Id
    // ^ defined: 1
      @Id()
    // ^ defined: 1
      @Id(y)
    // ^ defined: 1
    //    ^ defined: 15
    f: int;
}

export {};
