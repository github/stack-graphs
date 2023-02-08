class A {
  void f() {
    return f() + f();
    //     ^ defined: 2
    //           ^ defined: 2
  }
}
