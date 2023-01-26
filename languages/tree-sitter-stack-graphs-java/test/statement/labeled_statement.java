class A {
  void f() {
    x: f();
    // ^ defined: 2
  }
}
