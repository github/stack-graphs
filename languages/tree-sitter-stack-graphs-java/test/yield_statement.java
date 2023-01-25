class A {
  void f() {
    yield f();
    //    ^ defined: 2
  }
}
