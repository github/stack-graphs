class A {
  void f() {
    while (f()) {
      //   ^ defined: 2
        f();
      //^ defined: 2
    }
  }
}
