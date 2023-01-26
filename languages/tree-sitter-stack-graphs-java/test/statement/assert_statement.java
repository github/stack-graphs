class A {
  void f() {
    assert f();
    //     ^ defined: 2
    assert f(): f();
    //     ^ defined: 2
    //          ^ defined: 2
  }
}
