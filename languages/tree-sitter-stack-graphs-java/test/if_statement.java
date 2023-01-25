class A {
  void f() {
    boolean x;
    if (x) {
      //^ defined: 3
        f();
      //^ defined: 2
    }
    if (true) {} else {
        f();
      //^ defined: 2
    }
  }
}
