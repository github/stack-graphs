class A {
  void f() {
    int x;
    switch (x) {
      //    ^ defined: 3
      default:
        f();
      //^ defined: 2
    }
  }
}
