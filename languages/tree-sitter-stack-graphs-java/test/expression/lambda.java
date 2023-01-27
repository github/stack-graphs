class A {
  void f() {
    x -> f(x);
    //   ^ defined: 2
    //     ^ defined: 3
  }
}
