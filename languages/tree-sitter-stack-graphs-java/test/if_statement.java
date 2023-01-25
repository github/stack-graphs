class A {
  void f() {
    boolean x;
    if (x) {
      //^ defined: 3
        f();
      //^ defined: 2
    } else {
        f();
      //^ defined: 2
    }
  }
}
